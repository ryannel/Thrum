//! CPU coverage of the scene attempt driver and ordered candidate routing.

use super::*;
use crate::contribution::{
    ContributionFrame, EffectCadence, EffectGraph, EffectPass, SceneContributions, SceneSurface,
};
use std::sync::{Arc, Mutex};

type Events = Arc<Mutex<Vec<Event>>>;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Attempt(RayStructureSelection),
    Prepare(u8, u32),
    Abort(u8),
    Commit(u8),
    Rebuild(u8, u32),
}

struct Legacy {
    id: u8,
    value: u32,
    events: Events,
}

impl Legacy {
    fn rebuild_state(&mut self) -> Result<(), FrameError> {
        self.events
            .lock()
            .unwrap()
            .push(Event::Rebuild(self.id, self.value));
        self.value += 1;
        if self.id == 2 {
            Err(FrameError::Scene("legacy member refused".into()))
        } else {
            Ok(())
        }
    }
}

impl SceneContribution for Legacy {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }
    fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), String> {
        self.rebuild_state().map_err(|error| error.to_string())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

struct Member(Legacy);

impl SceneContribution for Member {
    fn supports_candidate_preparation(&self) -> bool {
        true
    }
    fn passes(&self) -> ContributionPasses {
        self.0.passes()
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.0.rebuild(surface)
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
    fn effect_runs(&self, index: usize, _: &ContributionFrame) -> bool {
        index == 0
    }
}

struct Staged {
    id: u8,
    committed: bool,
    events: Events,
}

impl Drop for Staged {
    fn drop(&mut self) {
        if !self.committed {
            self.events
                .lock()
                .expect("test record")
                .push(Event::Abort(self.id));
        }
    }
}

impl Staged {
    fn install(mut self, member: &mut Member) {
        self.committed = true;
        member.0.value += 1;
        self.events
            .lock()
            .expect("test record")
            .push(Event::Commit(self.id));
    }
}

fn mock_prepare(
    member: &dyn SceneContribution,
    failure: Option<u8>,
) -> Result<ContributionCandidate, FrameError> {
    let member: &dyn std::any::Any = member;
    if let Some(list) = member.downcast_ref::<SceneContributions>() {
        return list.prepare_members(|child| mock_prepare(child, failure));
    }
    let member = (member as &dyn std::any::Any)
        .downcast_ref::<Member>()
        .unwrap();
    let Legacy { id, value, events } = &member.0;
    events
        .lock()
        .expect("test record")
        .push(Event::Prepare(*id, *value));
    if failure == Some(*id) {
        return Err(FrameError::Scene("member preparation refused".into()));
    }
    let staged = Staged {
        id: *id,
        committed: false,
        events: Arc::clone(events),
    };
    let graph = EffectGraph {
        passes: vec![EffectPass {
            name: format!("member {id}"),
            cadence: EffectCadence::EveryFrame,
            async_eligible: false,
            accesses: Vec::new(),
        }],
        ..EffectGraph::default()
    };
    Ok(ContributionCandidate::new(
        member.passes(),
        graph,
        move |owner: &mut Member| staged.install(owner),
    ))
}

fn member(events: &Events, id: u8) -> Box<dyn SceneContribution> {
    Box::new(Member(Legacy {
        id,
        value: 0,
        events: Arc::clone(events),
    }))
}

fn three_members(events: &Events) -> SceneContributions {
    SceneContributions::new(vec![
        member(events, 1),
        Box::new(SceneContributions::new(vec![member(events, 2)])),
        member(events, 3),
    ])
}

#[test]
fn nested_legacy_member_routes_directly_to_maps() {
    use RayStructureSelection::{Enabled, NotRequested, UnsupportedContribution};

    for (request, expected) in [
        (Enabled, UnsupportedContribution),
        (NotRequested, NotRequested),
    ] {
        let events = Events::default();
        let legacy = |id| -> Box<dyn SceneContribution> {
            Box::new(Legacy {
                id,
                value: 7,
                events: Arc::clone(&events),
            })
        };
        let mut list = SceneContributions::new(vec![
            legacy(1),
            Box::new(SceneContributions::new(vec![legacy(2)])),
            legacy(3),
        ]);
        let selection = contribution_admission(request, supports_candidates(Some(&list)));
        assert_eq!(selection, expected);
        let result = pollster::block_on(prepare_3d_transition(selection, async |attempt| {
            events
                .lock()
                .expect("test record")
                .push(Event::Attempt(attempt));
            rebuild_legacy(&mut list)
        }));
        assert!(
            result.is_err(),
            "the ordinary Maps recovery receives the refusal"
        );
        assert_eq!(
            *events.lock().expect("test record"),
            [
                Event::Attempt(expected),
                Event::Rebuild(1, 7),
                Event::Rebuild(2, 7),
            ]
        );
    }
}

fn rebuild_legacy(member: &mut dyn SceneContribution) -> Result<(), FrameError> {
    let member: &mut dyn std::any::Any = member;
    if member.is::<SceneContributions>() {
        return member
            .downcast_mut::<SceneContributions>()
            .expect("test list")
            .rebuild_entries(rebuild_legacy);
    }
    member
        .downcast_mut::<Legacy>()
        .expect("legacy test member")
        .rebuild_state()
}

fn assert_routing(list: &SceneContributions, routed: bool) {
    let frame = ContributionFrame {
        time: crate::contribution::EffectTime {
            epoch: 1,
            timeline: 1,
            tick: 0,
            tick_duration: std::time::Duration::from_millis(16),
            alpha: 0.0,
            display_phase: 0.0,
            capture: false,
        },
        exposure_side: 0,
        environment: None,
        view_proj: [[0.0; 4]; 4],
        view_uniform_offset: 0,
        camera: [0.0; 3],
        generation: 1,
        history_reset: false,
    };
    for index in 0..3 {
        assert_eq!(list.effect_runs(index, &frame), routed, "effect {index}");
    }
    assert!(!list.effect_runs(3, &frame));
}

#[test]
fn scene_and_list_commit_only_after_the_complete_attempt() {
    use RayStructureSelection::{Enabled, PreparationFailed};

    for (leaf, final_validation, fail_maps) in [
        (Some(1), false, false),
        (Some(2), false, false),
        (Some(3), false, false),
        (None, true, false),
        (None, true, true),
        (None, false, false),
    ] {
        let events = Events::default();
        let mut list = three_members(&events);
        let mut accepted_scene = 9;
        assert!(supports_candidates(Some(&list)));
        let result = pollster::block_on(prepare_3d_transition(Enabled, async |attempt| {
            events
                .lock()
                .expect("test record")
                .push(Event::Attempt(attempt));
            let failure = if attempt == Enabled {
                leaf
            } else {
                fail_maps.then_some(3)
            };
            let mut candidate = mock_prepare(&list, failure)?;
            if final_validation && attempt == Enabled {
                candidate
                    .graph
                    .passes
                    .push(candidate.graph.passes[0].clone());
            }
            candidate.graph.validate().map_err(FrameError::Scene)?;
            Ok((attempt, candidate))
        }));
        assert_eq!(accepted_scene, 9);
        assert_routing(&list, false);
        assert!(
            !events
                .lock()
                .unwrap()
                .iter()
                .any(|event| matches!(event, Event::Commit(_)))
        );
        let retry = leaf.is_some() || final_validation;
        {
            let events = events.lock().expect("test record");
            let start = events
                .iter()
                .position(|event| *event == Event::Attempt(PreparationFailed));
            assert_eq!(start.is_some(), retry);
            if let Some(start) = start {
                let staged_count = leaf.map_or(3, |failure| failure - 1);
                for id in 1..=staged_count {
                    assert!(events[..start].contains(&Event::Abort(id)));
                }
                assert_eq!(
                    &events[start + 1..start + 4],
                    [
                        Event::Prepare(1, 0),
                        Event::Prepare(2, 0),
                        Event::Prepare(3, 0),
                    ]
                );
            }
        }
        if fail_maps {
            assert!(result.is_err());
            // A later successful attempt still observes the original member values.
            events.lock().expect("test record").clear();
            drop(mock_prepare(&list, None).expect("all leaves can prepare"));
            assert_eq!(
                &events.lock().expect("test record")[..3],
                [
                    Event::Prepare(1, 0),
                    Event::Prepare(2, 0),
                    Event::Prepare(3, 0),
                ]
            );
        } else {
            let (attempt, candidate) = result.expect("successful scene attempt");
            assert_eq!(attempt, if retry { PreparationFailed } else { Enabled });
            accepted_scene = 10;
            candidate.commit(&mut list);
            let events = events.lock().expect("test record");
            assert_eq!(
                &events[events.len() - 3..],
                [Event::Commit(1), Event::Commit(2), Event::Commit(3),]
            );
            assert_eq!(accepted_scene, 10);
            assert_routing(&list, true);
        }
    }
}

#[test]
fn empty_list_and_standalone_member_admit_and_prepare() {
    let events = Events::default();
    let empty: Box<dyn SceneContribution> = Box::new(SceneContributions::default());
    for mut owner in [empty, member(&events, 1)] {
        assert!(supports_candidates(Some(owner.as_ref())));
        let (selection, candidate) = pollster::block_on(prepare_3d_transition(
            RayStructureSelection::Enabled,
            async |selection| Ok((selection, mock_prepare(owner.as_ref(), None)?)),
        ))
        .expect("owner can prepare");
        assert_eq!(selection, RayStructureSelection::Enabled);
        candidate.commit(owner.as_mut());
    }
    assert_eq!(
        *events.lock().expect("test record"),
        [Event::Prepare(1, 0), Event::Commit(1)]
    );
}
