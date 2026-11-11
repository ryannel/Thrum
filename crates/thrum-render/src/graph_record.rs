//! Prepared pass recorders. Binding pays for lookup and validation once; frame
//! execution walks parallel arrays in the graph's already compiled order.

use crate::graph::{CompiledGraph, CompiledPass, GraphError, PassKind};

/// The borrowed frame context and error type supplied by one renderer family.
/// A GAT keeps encoder and target borrows out of the retained graph's lifetime.
pub(crate) trait RecordFamily {
    type Frame<'a>;
    type Error;
}

type RecordFn<F> = Box<
    dyn for<'a> Fn(
            &mut <F as RecordFamily>::Frame<'a>,
            &CompiledPass,
        ) -> Result<(), <F as RecordFamily>::Error>
        + Send
        + Sync,
>;

/// One closure for a stable pass declaration, allocated only at graph binding.
pub(crate) struct RecordBinding<F: RecordFamily> {
    pass: usize,
    kind: PassKind,
    record: RecordFn<F>,
}

impl<F: RecordFamily> RecordBinding<F> {
    pub(crate) fn new(
        pass: usize,
        kind: PassKind,
        record: impl for<'a> Fn(&mut F::Frame<'a>, &CompiledPass) -> Result<(), F::Error>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            pass,
            kind,
            record: Box::new(record),
        }
    }
}

/// Live, validated closures retained beside one compiled GPU plan.
pub(crate) struct ExecutableGraph<F: RecordFamily> {
    plan: CompiledGraph,
    recorders: Vec<RecordFn<F>>,
}

impl CompiledGraph {
    /// Bind exactly one recorder for each live pass. Dead declarations have no
    /// recorder and consume no frame work.
    ///
    /// # Errors
    ///
    /// Refuses an unknown, duplicate, missing or wrong-kind recorder.
    pub(crate) fn bind<F: RecordFamily>(
        self,
        bindings: Vec<RecordBinding<F>>,
    ) -> Result<ExecutableGraph<F>, GraphError> {
        let slots = self.pass_order.iter().copied().max().map_or(0, |id| id + 1);
        let mut by_declaration: Vec<Option<RecordBinding<F>>> =
            std::iter::repeat_with(|| None).take(slots).collect();
        for binding in bindings {
            let id = binding.pass;
            if !self.pass_order.contains(&id) {
                return Err(GraphError::UnknownRecorder { pass: id });
            }
            let slot = &mut by_declaration[id];
            if slot.is_some() {
                return Err(GraphError::DuplicateRecorder { pass: id });
            }
            *slot = Some(binding);
        }
        let mut recorders = Vec::with_capacity(self.pass_order.len());
        for (id, pass) in self.pass_bindings() {
            let binding = by_declaration[id]
                .take()
                .ok_or(GraphError::MissingRecorder { pass: id })?;
            if pass.kind != Some(binding.kind) {
                return Err(GraphError::RecorderKindMismatch { pass: id });
            }
            recorders.push(binding.record);
        }
        Ok(ExecutableGraph {
            plan: self,
            recorders,
        })
    }
}

impl<F: RecordFamily> ExecutableGraph<F> {
    pub(crate) fn execute_before_acquire(&self, frame: &mut F::Frame<'_>) -> Result<(), F::Error> {
        let end = self.plan.pre_acquire_passes.unwrap_or(self.recorders.len());
        for (pass, record) in self.plan.pass_bindings().zip(&self.recorders).take(end) {
            record(frame, pass.1)?;
        }
        Ok(())
    }

    pub(crate) fn execute_after_acquire(&self, frame: &mut F::Frame<'_>) -> Result<(), F::Error> {
        let start = self.plan.pre_acquire_passes.unwrap_or(self.recorders.len());
        for (pass, record) in self.plan.pass_bindings().zip(&self.recorders).skip(start) {
            record(frame, pass.1)?;
        }
        Ok(())
    }
    /// Run already bound closures in the plan's topological order.
    pub(crate) fn execute(&self, frame: &mut F::Frame<'_>) -> Result<(), F::Error> {
        for (pass, record) in self.plan.pass_bindings().zip(&self.recorders) {
            record(frame, pass.1)?;
        }
        Ok(())
    }

    /// Access the closed plan for timing capacity and transient resources.
    pub(crate) fn plan(&self) -> &CompiledGraph {
        &self.plan
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::graph::{
        Access, AccessKind, FrameGraph, GraphError, Pass, PassKind, PassScope, ResourceDescriptor,
        ResourceScope,
    };

    use super::{RecordBinding, RecordFamily};

    mod split_tests;

    struct TestFamily;

    #[derive(Default)]
    struct Frame {
        surface_bound: bool,
        order: [usize; 2],
        written: usize,
    }

    impl RecordFamily for TestFamily {
        type Frame<'a> = Frame;
        type Error = ();
    }

    fn copy_plan() -> crate::graph::CompiledGraph {
        let mut graph = FrameGraph::default();
        let unused = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 8,
                    usage: wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Transient,
            )
            .expect("unused buffer");
        let live = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 8,
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Transient,
            )
            .expect("live buffer");
        for (name, resource, kind, output) in [
            ("dead", unused, AccessKind::CopyWrite, false),
            ("write", live.clone(), AccessKind::CopyWrite, false),
            ("read", live, AccessKind::CopyRead, true),
        ] {
            graph.pass(Pass {
                name: name.to_owned(),
                scope: PassScope::Shared,
                pipeline: None,
                accesses: vec![Access {
                    resource,
                    range: 0..8,
                    kind,
                    store: true,
                }],
                external_side_effect: output,
            });
        }
        graph.compile(1, &BTreeSet::new()).expect("copy closure")
    }

    fn binding(pass: usize) -> RecordBinding<TestFamily> {
        RecordBinding::new(pass, PassKind::Copy, move |frame: &mut Frame, _| {
            frame.order[frame.written] = pass;
            frame.written += 1;
            Ok(())
        })
    }

    #[test]
    fn bound_closures_follow_live_compiled_order_without_frame_lookup() {
        let graph = copy_plan();
        assert_eq!(graph.pass_order, [1, 2]);
        let executable = graph
            .bind(vec![binding(2), binding(1)])
            .expect("one per live pass");
        let mut frame = Frame::default();
        executable.execute(&mut frame).expect("record");
        assert_eq!(frame.order, [1, 2]);
        assert_eq!(frame.written, 2);
        assert_eq!(executable.plan().culled_passes, 1);
    }

    #[test]
    fn execution_stops_before_later_passes_when_a_recorder_fails() {
        let first = RecordBinding::<TestFamily>::new(1, PassKind::Copy, |frame: &mut Frame, _| {
            frame.written += 1;
            Err(())
        });
        let second = RecordBinding::<TestFamily>::new(2, PassKind::Copy, |frame: &mut Frame, _| {
            frame.written += 1;
            Ok(())
        });
        let executable = copy_plan()
            .bind(vec![first, second])
            .expect("valid bindings");
        let mut frame = Frame::default();
        assert_eq!(executable.execute(&mut frame), Err(()));
        assert_eq!(frame.written, 1);
    }

    #[test]
    fn binder_refuses_missing_duplicate_dead_and_wrong_kind_recorders() {
        assert!(matches!(
            copy_plan().bind(vec![binding(1)]),
            Err(GraphError::MissingRecorder { pass: 2 })
        ));
        assert!(matches!(
            copy_plan().bind(vec![binding(1), binding(1), binding(2)]),
            Err(GraphError::DuplicateRecorder { pass: 1 })
        ));
        assert!(matches!(
            copy_plan().bind(vec![binding(0), binding(1), binding(2)]),
            Err(GraphError::UnknownRecorder { pass: 0 })
        ));
        let wrong = RecordBinding::new(1, PassKind::Render, |_: &mut Frame, _| Ok(()));
        assert!(matches!(
            copy_plan().bind(vec![wrong, binding(2)]),
            Err(GraphError::RecorderKindMismatch { pass: 1 })
        ));
    }
}
