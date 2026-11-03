//! Object-scoped strict decoding for the inactive shared render source.
//!
//! F3 and F4 will use one decoder with different projections. Mesh scratch
//! lives only through its callback; F3 retains at most its compact extrema
//! and identity, while F4 may copy the geometry into a product preparation.
//! Operation-wide charged allocation and diagnostics are separate work.

use serde_json::value::RawValue;

use crate::render_geometry_exact::{
    NonFinitePosition, TriangleSourceError, position_within_local_radius, triangle_is_nondegenerate,
};
use crate::render_json::{SourceText, parse_raw_object, uuid_bytes};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MeshBounds {
    pub(crate) id: [u8; 16],
    pub(crate) local_min: [f64; 3],
    pub(crate) local_max: [f64; 3],
}

pub(crate) struct MeshEvent<'a> {
    pub(crate) bounds: MeshBounds,
    pub(crate) positions_m: &'a [[f64; 3]],
    pub(crate) uv0: &'a [[f64; 2]],
    pub(crate) triangles: &'a [[u32; 3]],
}

#[derive(Debug)]
pub(crate) enum MeshDecodeError<E> {
    Structural {
        id: Option<[u8; 16]>,
        field: MeshField,
        problem: MeshProblem,
    },
    Projection {
        id: [u8; 16],
        cause: E,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum MeshField {
    Id,
    Positions,
    Uv0,
    Triangles,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum MeshProblem {
    Json(serde_json::Error),
    Missing,
    Duplicate,
    Unknown(String),
    InvalidId,
    PositionNonfinite {
        vertex: usize,
    },
    PositionOutsideLocalRadius {
        vertex: usize,
    },
    UvCount {
        positions: usize,
        uv0: usize,
    },
    UvNonfinite {
        vertex: usize,
    },
    TriangleIndex {
        triangle: usize,
        cause: TriangleSourceError,
    },
    TriangleDegenerate {
        triangle: usize,
    },
}

const MESH_FIELDS: [&str; 4] = ["id", "positions_m", "uv0", "triangles"];
const MESH_FIELD_ORDER: [MeshField; 4] = [
    MeshField::Id,
    MeshField::Positions,
    MeshField::Uv0,
    MeshField::Triangles,
];

fn structural<E>(
    id: Option<[u8; 16]>,
    field: MeshField,
    problem: MeshProblem,
) -> MeshDecodeError<E> {
    MeshDecodeError::Structural { id, field, problem }
}

pub(crate) fn decode_mesh_v1<E>(
    raw: &RawValue,
    project: impl FnOnce(MeshEvent<'_>) -> Result<(), E>,
) -> Result<MeshBounds, MeshDecodeError<E>> {
    let fields = parse_raw_object(raw.get(), &MESH_FIELDS)
        .map_err(|source| structural(None, MeshField::Id, MeshProblem::Json(source)))?;
    let decoded_id: Option<Result<SourceText<'_>, _>> =
        fields.values[0].map(|value| serde_json::from_str(value.get()));
    // A repeated decoded ID has no unique semantic owner. Report its
    // collection/ordinal fallback rather than whichever value appeared last.
    let possible_id = if fields.duplicate[0] {
        None
    } else {
        decoded_id
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .and_then(|text| uuid_bytes(&text.0))
    };
    if let Some(index) = fields.duplicate.iter().position(|&duplicate| duplicate) {
        return Err(structural(
            possible_id,
            MESH_FIELD_ORDER[index],
            MeshProblem::Duplicate,
        ));
    }
    if let Some(unknown) = fields.unknown {
        return Err(structural(
            possible_id,
            MeshField::Unknown,
            MeshProblem::Unknown(unknown.0.into_owned()),
        ));
    }
    let decoded_id =
        decoded_id.ok_or_else(|| structural(None, MeshField::Id, MeshProblem::Missing))?;
    let source_id =
        decoded_id.map_err(|source| structural(None, MeshField::Id, MeshProblem::Json(source)))?;
    let id = uuid_bytes(&source_id.0)
        .ok_or_else(|| structural(None, MeshField::Id, MeshProblem::InvalidId))?;
    let positions_raw = fields.values[1]
        .ok_or_else(|| structural(Some(id), MeshField::Positions, MeshProblem::Missing))?;
    let positions_m: Vec<[f64; 3]> = serde_json::from_str(positions_raw.get())
        .map_err(|source| structural(Some(id), MeshField::Positions, MeshProblem::Json(source)))?;
    let mut local_min = [f64::INFINITY; 3];
    let mut local_max = [f64::NEG_INFINITY; 3];
    for (vertex, position) in positions_m.iter().copied().enumerate() {
        match position_within_local_radius(position) {
            Ok(true) => {}
            Ok(false) => {
                return Err(structural(
                    Some(id),
                    MeshField::Positions,
                    MeshProblem::PositionOutsideLocalRadius { vertex },
                ));
            }
            Err(NonFinitePosition) => {
                return Err(structural(
                    Some(id),
                    MeshField::Positions,
                    MeshProblem::PositionNonfinite { vertex },
                ));
            }
        }
        for axis in 0..3 {
            local_min[axis] = local_min[axis].min(position[axis]);
            local_max[axis] = local_max[axis].max(position[axis]);
        }
    }
    if positions_m.is_empty() {
        local_min = [0.0; 3];
        local_max = [0.0; 3];
    }
    let uv_raw = fields.values[2]
        .ok_or_else(|| structural(Some(id), MeshField::Uv0, MeshProblem::Missing))?;
    let uv0: Vec<[f64; 2]> = serde_json::from_str(uv_raw.get())
        .map_err(|source| structural(Some(id), MeshField::Uv0, MeshProblem::Json(source)))?;
    if uv0.len() != positions_m.len() {
        return Err(structural(
            Some(id),
            MeshField::Uv0,
            MeshProblem::UvCount {
                positions: positions_m.len(),
                uv0: uv0.len(),
            },
        ));
    }
    for (vertex, uv) in uv0.iter().enumerate() {
        if uv.iter().any(|value| !value.is_finite()) {
            return Err(structural(
                Some(id),
                MeshField::Uv0,
                MeshProblem::UvNonfinite { vertex },
            ));
        }
    }
    let triangles_raw = fields.values[3]
        .ok_or_else(|| structural(Some(id), MeshField::Triangles, MeshProblem::Missing))?;
    let triangles: Vec<[u32; 3]> = serde_json::from_str(triangles_raw.get())
        .map_err(|source| structural(Some(id), MeshField::Triangles, MeshProblem::Json(source)))?;
    for (triangle, indices) in triangles.iter().copied().enumerate() {
        match triangle_is_nondegenerate(&positions_m, indices) {
            Ok(true) => {}
            Ok(false) => {
                return Err(structural(
                    Some(id),
                    MeshField::Triangles,
                    MeshProblem::TriangleDegenerate { triangle },
                ));
            }
            Err(cause) => {
                return Err(structural(
                    Some(id),
                    MeshField::Triangles,
                    MeshProblem::TriangleIndex { triangle, cause },
                ));
            }
        }
    }
    let bounds = MeshBounds {
        id,
        local_min,
        local_max,
    };
    project(MeshEvent {
        bounds,
        positions_m: &positions_m,
        uv0: &uv0,
        triangles: &triangles,
    })
    .map_err(|cause| MeshDecodeError::Projection { id, cause })?;
    Ok(bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_json::visit_raw_array;

    const MESH: &str = r#"{"id":"a6b9c300-0000-4000-8000-000000000014","positions_m":[[0,0,0],[1,0,0],[0,1,0]],"uv0":[[0,0],[1,0],[0,1]],"triangles":[[0,1,2]]}"#;

    fn raw(text: &str) -> &RawValue {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn checked_mesh_borrows_object_scratch_and_returns_tight_bounds() {
        let mut called = false;
        let bounds = decode_mesh_v1(raw(MESH), |event| -> Result<(), ()> {
            assert_eq!(event.positions_m.len(), 3);
            assert_eq!(event.uv0.len(), 3);
            assert_eq!(event.triangles, &[[0, 1, 2]]);
            called = true;
            Ok(())
        })
        .unwrap();
        assert!(called);
        assert_eq!(bounds.id[15], 20);
        assert_eq!(bounds.local_min, [0.0, 0.0, 0.0]);
        assert_eq!(bounds.local_max, [1.0, 1.0, 0.0]);
        assert!(matches!(
            decode_mesh_v1(raw(MESH), |_| Err("pressure")),
            Err(MeshDecodeError::Projection {
                cause: "pressure",
                ..
            })
        ));
    }

    #[test]
    fn unreferenced_outlier_and_exact_triangle_rules_refuse() {
        let outlier = MESH
            .replace("[0,1,0]]", "[0,1,0],[1000,1000,0]]")
            .replace("[0,1]]", "[0,1],[0,0]]");
        assert!(matches!(
            decode_mesh_v1(raw(&outlier), |_| Ok::<_, ()>(())),
            Err(MeshDecodeError::Structural {
                field: MeshField::Positions,
                problem: MeshProblem::PositionOutsideLocalRadius { vertex: 3 },
                ..
            })
        ));
        let repeated = MESH.replace("[0,1,2]", "[0,1,1]");
        assert!(matches!(
            decode_mesh_v1(raw(&repeated), |_| Ok::<_, ()>(())),
            Err(MeshDecodeError::Structural {
                field: MeshField::Triangles,
                problem: MeshProblem::TriangleIndex {
                    cause: TriangleSourceError::RepeatedIndex,
                    ..
                },
                ..
            })
        ));
        let flat = MESH.replace("[0,1,0]", "[2,0,0]");
        assert!(matches!(
            decode_mesh_v1(raw(&flat), |_| Ok::<_, ()>(())),
            Err(MeshDecodeError::Structural {
                field: MeshField::Triangles,
                problem: MeshProblem::TriangleDegenerate { triangle: 0 },
                ..
            })
        ));
    }

    #[test]
    fn empty_mesh_and_projection_refusal_are_distinct() {
        let empty = MESH
            .replace("[[0,0,0],[1,0,0],[0,1,0]]", "[]")
            .replace("[[0,0],[1,0],[0,1]]", "[]")
            .replace("[[0,1,2]]", "[]");
        let bounds = decode_mesh_v1(raw(&empty), |_| Ok::<_, ()>(())).unwrap();
        assert_eq!(bounds.local_min, [0.0; 3]);
        assert_eq!(bounds.local_max, [0.0; 3]);
        let positional = r#"["a6b9c300-0000-4000-8000-000000000014",[],[],[]]"#;
        assert!(matches!(
            decode_mesh_v1(raw(positional), |_| Ok::<_, ()>(())),
            Err(MeshDecodeError::Structural {
                problem: MeshProblem::Json(_),
                ..
            })
        ));
    }

    #[test]
    fn field_order_does_not_hide_id_or_earlier_schema_error() {
        let id = "a6b9c300-0000-4000-8000-000000000014";
        for text in [
            format!(r#"{{"id":"{id}","positions_m":"wrong","uv0":"wrong","triangles":[]}}"#),
            format!(r#"{{"triangles":[],"uv0":"wrong","positions_m":"wrong","id":"{id}"}}"#),
        ] {
            assert!(matches!(
                decode_mesh_v1(raw(&text), |_| Ok::<_, ()>(())),
                Err(MeshDecodeError::Structural {
                    id: Some(found),
                    field: MeshField::Positions,
                    problem: MeshProblem::Json(_),
                }) if found[15] == 20
            ));
        }
        let duplicate = format!(
            r#"{{"id":"{id}","positions_m":"wrong","uv0":[],"uv\u0030":[],"triangles":[]}}"#
        );
        assert!(matches!(
            decode_mesh_v1(raw(&duplicate), |_| Ok::<_, ()>(())),
            Err(MeshDecodeError::Structural {
                id: Some(found),
                field: MeshField::Uv0,
                problem: MeshProblem::Duplicate,
            }) if found[15] == 20
        ));
    }

    #[test]
    fn repeated_id_uses_anonymous_locator_under_value_reorder() {
        let first = "a6b9c300-0000-4000-8000-000000000014";
        let second = "a6b9c300-0000-4000-8000-000000000015";
        for ids in [
            format!(r#""id":"{first}","id":"{second}""#),
            format!(r#""id":"{second}","id":"{first}""#),
            format!(r#""id":"{first}","i\u0064":"invalid""#),
            format!(r#""id":"invalid","id":"{first}""#),
        ] {
            let object = format!("{{{ids}}}");
            assert!(matches!(
                decode_mesh_v1(raw(&object), |_| Ok::<_, ()>(())),
                Err(MeshDecodeError::Structural {
                    id: None,
                    field: MeshField::Id,
                    problem: MeshProblem::Duplicate,
                })
            ));
        }
    }

    #[test]
    fn streamed_collection_releases_each_object_after_projection() {
        let reordered = MESH.replace(
            r#"{"id":"a6b9c300-0000-4000-8000-000000000014","positions_m":#,
            r#"{"triangles":[[0,1,2]],"id":"a6b9c300-0000-4000-8000-000000000014","positions_m":#,
        ).replace(r#","triangles":[[0,1,2]]}"#,
            "}",
        );
        let source = format!("[{MESH},{reordered}]");
        let mut bounds = Vec::new();
        visit_raw_array(raw(&source), |item| {
            let found = decode_mesh_v1(item, |_| Ok::<_, ()>(()))?;
            bounds.push(found);
            Ok::<_, MeshDecodeError<()>>(())
        })
        .unwrap();
        assert_eq!(bounds.len(), 2);
        assert_eq!(bounds[0], bounds[1]);
    }
}
