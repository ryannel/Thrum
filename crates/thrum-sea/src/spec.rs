//! One typed resource/pass description drives allocation, binding, dispatch and graph access.
use crate::{CACHE_SIZE, LEVELS, MAX_QUERIES, PACKED_CHANNELS, PIXELS, TRANSFORM_SIZE, records};
use thrum_render::contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass};
use thrum_render::graph::{AccessKind, ResourceDescriptor, ResourceScope};

pub(super) const CASCADES: u32 = 3;
pub(super) const MAX_LONG: usize = 192;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum BufferKey {
    Params,
    Modes,
    Long,
    Queries,
    TransformA,
    TransformB,
    TransformFinal,
    Twiddles,
    Cascades,
    LongField,
    UploadStaging,
}

pub(super) struct Resource {
    pub key: BufferKey,
    pub name: String,
    pub size: u64,
    pub usage: wgpu::BufferUsages,
    pub scope: ResourceScope,
}

pub(super) fn resources() -> Vec<Resource> {
    use BufferKey as B;
    let work = u64::from(CASCADES * PACKED_CHANNELS * PIXELS) * size_of::<[f32; 2]>() as u64;
    let mut rows = vec![
        (
            B::Params,
            "sea parameters".into(),
            size_of::<records::Params>() as u64,
        ),
        (
            B::Modes,
            "sea spectrum".into(),
            u64::from(CASCADES * PIXELS) * size_of::<records::Bin>() as u64,
        ),
        (
            B::Long,
            "sea long components".into(),
            (MAX_LONG * size_of::<records::Long>()) as u64,
        ),
        (
            B::Queries,
            "sea query points".into(),
            u64::from(MAX_QUERIES) * size_of::<records::Query>() as u64,
        ),
        (B::TransformA, "sea transform A".into(), work),
        (B::TransformB, "sea transform B".into(), work),
        (B::TransformFinal, "sea final transform".into(), work),
        (
            B::Twiddles,
            "sea transform twiddles".into(),
            u64::from(TRANSFORM_SIZE / 2) * size_of::<[f32; 2]>() as u64,
        ),
        (
            B::Cascades,
            "sea cascades".into(),
            u64::from(CASCADES * PIXELS) * size_of::<records::Field>() as u64,
        ),
        (
            B::LongField,
            "sea long field".into(),
            u64::from(MAX_QUERIES + LEVELS * CACHE_SIZE * CACHE_SIZE)
                * size_of::<records::Field>() as u64,
        ),
    ];
    let staging_size = rows
        .iter()
        .filter(|(key, _, _)| matches!(key, B::Params | B::Long | B::Queries))
        .map(|(_, _, size)| size)
        .sum();
    rows.push((B::UploadStaging, "sea upload staging".into(), staging_size));
    rows.into_iter()
        .map(|(key, name, size)| {
            let uniform = key == B::Params;
            Resource {
                key,
                name,
                size,
                usage: if key == B::UploadStaging {
                    wgpu::BufferUsages::COPY_SRC
                } else if uniform {
                    wgpu::BufferUsages::UNIFORM
                } else {
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC
                } | wgpu::BufferUsages::COPY_DST,
                scope: if matches!(
                    key,
                    B::Params | B::Modes | B::Long | B::Queries | B::Twiddles | B::UploadStaging
                ) {
                    ResourceScope::Imported { initialized: true }
                } else {
                    ResourceScope::Transient
                },
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PassKey {
    Long,
    Evolve,
    Rows,
    Columns,
    Unpack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PipelineKey {
    Long,
    Evolve,
    Rows,
    Columns,
    Unpack,
}

pub(super) struct Binding {
    pub group: GroupKey,
    pub slot: u32,
    pub buffer: BufferKey,
    pub access: AccessKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum GroupKey {
    Fields,
}

impl GroupKey {
    pub(super) const ALL: [Self; 1] = [Self::Fields];
    pub(super) const fn slot(self) -> u32 {
        match self {
            Self::Fields => 0,
        }
    }
}

pub(super) struct PassSpec {
    pub key: PassKey,
    pub name: String,
    pub entry: &'static str,
    pub bindings: Vec<Binding>,
    pub dispatch: [u32; 3],
}

impl PassSpec {
    pub(super) fn pipeline(&self) -> PipelineKey {
        match self.key {
            PassKey::Long => PipelineKey::Long,
            PassKey::Evolve => PipelineKey::Evolve,
            PassKey::Rows => PipelineKey::Rows,
            PassKey::Columns => PipelineKey::Columns,
            PassKey::Unpack => PipelineKey::Unpack,
        }
    }
    fn new(
        key: PassKey,
        name: &str,
        entry: &'static str,
        bindings: &[(u32, BufferKey, AccessKind)],
        dispatch: [u32; 3],
    ) -> Self {
        Self {
            key,
            name: name.into(),
            entry,
            bindings: bindings
                .iter()
                .map(|&(slot, buffer, access)| Binding {
                    group: GroupKey::Fields,
                    slot,
                    buffer,
                    access,
                })
                .collect(),
            dispatch,
        }
    }
}

pub(super) fn passes() -> Vec<PassSpec> {
    use AccessKind::{StorageRead as R, StorageWrite as W, UniformRead as U};
    use BufferKey as B;
    use PassKey as P;
    vec![
        PassSpec::new(
            P::Long,
            "sea long waves",
            "long_waves",
            &[
                (0, B::Params, U),
                (2, B::Long, R),
                (3, B::Queries, R),
                (8, B::LongField, W),
            ],
            [
                (MAX_QUERIES + LEVELS * CACHE_SIZE * CACHE_SIZE).div_ceil(64),
                1,
                1,
            ],
        ),
        PassSpec::new(
            P::Evolve,
            "sea cascade evolve",
            "evolve",
            &[(0, B::Params, U), (1, B::Modes, R), (4, B::TransformA, W)],
            [(CASCADES * PIXELS).div_ceil(64), 1, 1],
        ),
        PassSpec::new(
            P::Rows,
            "sea cascade rows",
            "rows",
            &[
                (4, B::TransformA, R),
                (5, B::TransformB, W),
                (9, B::Twiddles, R),
            ],
            [TRANSFORM_SIZE, CASCADES * PACKED_CHANNELS, 1],
        ),
        PassSpec::new(
            P::Columns,
            "sea cascade columns",
            "columns",
            &[
                (5, B::TransformB, R),
                (4, B::TransformFinal, W),
                (9, B::Twiddles, R),
            ],
            [TRANSFORM_SIZE, CASCADES * PACKED_CHANNELS, 1],
        ),
        PassSpec::new(
            P::Unpack,
            "sea cascade unpack",
            "unpack",
            &[(4, B::TransformFinal, R), (6, B::Cascades, W)],
            [(CASCADES * PIXELS).div_ceil(64), 1, 1],
        ),
    ]
}

pub(super) fn declaration(sizes: &[u64]) -> EffectGraph {
    let resources = resources();
    EffectGraph {
        view_volume_media: Vec::new(),
        sun_paths: Vec::new(),
        passes: passes()
            .iter()
            .map(|p| EffectPass {
                name: p.name.clone(),
                cadence: EffectCadence::EveryFrame,
                async_eligible: false,
                accesses: p
                    .bindings
                    .iter()
                    .map(|b| {
                        let resource = resources
                            .iter()
                            .position(|r| r.key == b.buffer)
                            .expect("declared buffer key");
                        EffectAccess {
                            resource,
                            range: 0..sizes[resource],
                            kind: b.access,
                            view: None,
                            history: None,
                        }
                    })
                    .collect(),
            })
            .collect(),
        resources: resources
            .into_iter()
            .zip(sizes)
            .map(|(r, &bytes)| {
                (
                    ResourceDescriptor::Buffer {
                        bytes,
                        usage: r.usage,
                    },
                    r.scope,
                )
            })
            .collect(),
        opaque_reads: Vec::new(),
        after_opaque: Vec::new(),
    }
}
