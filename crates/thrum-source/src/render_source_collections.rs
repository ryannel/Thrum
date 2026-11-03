//! Canonical structural error selection over borrowed render object arrays.
//!
//! Source structure outranks all later closure and F4 target checks. This
//! inactive layer keeps only the lowest error locator while walking each
//! collection; projections are provisional until the complete record passes.

use serde_json::value::RawValue;

use crate::render_json::{RawArrayError, visit_raw_array};
use crate::render_source_objects::{
    CameraDecodeError, CameraFacts, CameraField, CameraProblem, MeshDecodeError, MeshEvent,
    MeshField, MeshProblem, decode_camera_v1, decode_mesh_v1,
};

#[derive(Debug)]
pub(crate) enum CollectionScanError<E, F, P> {
    Json(serde_json::Error),
    Structural {
        ordinal: usize,
        id: Option<[u8; 16]>,
        field: F,
        problem: P,
    },
    Projection {
        ordinal: usize,
        id: [u8; 16],
        cause: E,
    },
}

pub(crate) type MeshScanError<E> = CollectionScanError<E, MeshField, MeshProblem>;
pub(crate) type CameraScanError<E> = CollectionScanError<E, CameraField, CameraProblem>;

struct Selected<'a, F, P> {
    ordinal: usize,
    id: Option<[u8; 16]>,
    field: F,
    problem: P,
    raw: &'a str,
}

fn before<F: Ord, P>(candidate: &Selected<'_, F, P>, current: &Selected<'_, F, P>) -> bool {
    match (candidate.id, current.id) {
        (Some(a), Some(b)) => a
            .cmp(&b)
            .then_with(|| candidate.field.cmp(&current.field))
            .then_with(|| candidate.raw.as_bytes().cmp(current.raw.as_bytes()))
            .then_with(|| candidate.ordinal.cmp(&current.ordinal))
            .is_lt(),
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => candidate
            .ordinal
            .cmp(&current.ordinal)
            .then_with(|| candidate.field.cmp(&current.field))
            .is_lt(),
    }
}

fn retain_lowest<'a, F: Ord, P>(
    best: &mut Option<Selected<'a, F, P>>,
    candidate: Selected<'a, F, P>,
) {
    if best
        .as_ref()
        .is_none_or(|current| before(&candidate, current))
    {
        *best = Some(candidate);
    }
}

fn finish<E, F, P>(best: Option<Selected<'_, F, P>>) -> Result<(), CollectionScanError<E, F, P>> {
    match best {
        Some(error) => Err(CollectionScanError::Structural {
            ordinal: error.ordinal,
            id: error.id,
            field: error.field,
            problem: error.problem,
        }),
        None => Ok(()),
    }
}

/// A successful callback is still provisional. On a structural error, the
/// caller discards all prior emitted state and this scan suppresses later
/// callbacks while looking for the lowest source error.
pub(crate) fn scan_meshes<E>(
    raw: &RawValue,
    mut project: impl FnMut(MeshEvent<'_>) -> Result<(), E>,
) -> Result<(), MeshScanError<E>> {
    let mut best = None;
    let mut ordinal = 0;
    let walked = visit_raw_array(raw, |item| {
        let here = ordinal;
        ordinal += 1;
        match decode_mesh_v1(item, |event| {
            if best.is_none() {
                project(event)
            } else {
                Ok(())
            }
        }) {
            Ok(_) => Ok(()),
            Err(MeshDecodeError::Structural { id, field, problem }) => {
                retain_lowest(
                    &mut best,
                    Selected {
                        ordinal: here,
                        id,
                        field,
                        problem,
                        raw: item.get(),
                    },
                );
                Ok(())
            }
            Err(MeshDecodeError::Projection { id, cause }) => {
                Err(CollectionScanError::Projection {
                    ordinal: here,
                    id,
                    cause,
                })
            }
        }
    });
    match walked {
        Err(RawArrayError::Json(source)) => Err(CollectionScanError::Json(source)),
        Err(RawArrayError::Visit(cause)) => Err(cause),
        Ok(()) => finish(best),
    }
}

/// Camera facts are small and copied into the provisional projection.
pub(crate) fn scan_cameras<E>(
    raw: &RawValue,
    mut project: impl FnMut(CameraFacts) -> Result<(), E>,
) -> Result<(), CameraScanError<E>> {
    let mut best = None;
    let mut ordinal = 0;
    let walked = visit_raw_array(raw, |item| {
        let here = ordinal;
        ordinal += 1;
        match decode_camera_v1(item) {
            Ok(facts) => {
                if best.is_some() {
                    Ok(())
                } else {
                    project(facts).map_err(|cause| CollectionScanError::Projection {
                        ordinal: here,
                        id: facts.id,
                        cause,
                    })
                }
            }
            Err(CameraDecodeError { id, field, problem }) => {
                retain_lowest(
                    &mut best,
                    Selected {
                        ordinal: here,
                        id,
                        field,
                        problem,
                        raw: item.get(),
                    },
                );
                Ok(())
            }
        }
    });
    match walked {
        Err(RawArrayError::Json(source)) => Err(CollectionScanError::Json(source)),
        Err(RawArrayError::Visit(cause)) => Err(cause),
        Ok(()) => finish(best),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOW: &str = "a6b9c300-0000-4000-8000-000000000001";
    const HIGH: &str = "a6b9c300-0000-4000-8000-000000000002";

    fn raw(text: &str) -> &RawValue {
        serde_json::from_str(text).unwrap()
    }

    fn mesh(id: &str, positions: &str, uv0: &str) -> String {
        format!(r#"{{"id":"{id}","positions_m":{positions},"uv0":{uv0},"triangles":[]}}"#)
    }

    fn camera(id: &str, position: &str) -> String {
        format!(
            r#"{{"id":"{id}","position_m":{position},"orientation_xyzw":[0,0,0,1],"vertical_fov_rad":1,"near_m":0.1,"far_m":20,"exposure":1}}"#
        )
    }

    #[test]
    fn mesh_errors_choose_uuid_then_schema_field_across_array_reorder() {
        let low_bad = mesh(LOW, r#""bad""#, "[]");
        let high_bad = mesh(HIGH, r#""bad""#, "[]");
        for source in [
            format!("[{low_bad},{high_bad}]"),
            format!("[{high_bad},{low_bad}]"),
        ] {
            assert!(matches!(
                scan_meshes(raw(&source), |_| Ok::<_, ()>(())),
                Err(CollectionScanError::Structural {
                    id: Some(id),
                    field: MeshField::Positions,
                    problem: MeshProblem::Json(_),
                    ..
                }) if id[15] == 1
            ));
        }
        let position_bad = mesh(LOW, r#""bad""#, "[]");
        let uv_bad = mesh(LOW, "[]", r#""bad""#);
        for source in [
            format!("[{position_bad},{uv_bad}]"),
            format!("[{uv_bad},{position_bad}]"),
        ] {
            assert!(matches!(
                scan_meshes(raw(&source), |_| Ok::<_, ()>(())),
                Err(CollectionScanError::Structural {
                    field: MeshField::Positions,
                    ..
                })
            ));
        }
    }

    #[test]
    fn same_uuid_and_field_use_borrowed_raw_tie_before_ordinal() {
        let first_vertex = mesh(LOW, "[[2000,0,0]]", "[[0,0]]");
        let second_vertex = mesh(LOW, "[[0,0,0],[2000,0,0]]", "[[0,0],[0,0]]");
        assert!(second_vertex < first_vertex);
        for source in [
            format!("[{first_vertex},{second_vertex}]"),
            format!("[{second_vertex},{first_vertex}]"),
        ] {
            assert!(matches!(
                scan_meshes(raw(&source), |_| Ok::<_, ()>(())),
                Err(CollectionScanError::Structural {
                    field: MeshField::Positions,
                    problem: MeshProblem::PositionOutsideLocalRadius { vertex: 1 },
                    ..
                })
            ));
        }
    }

    #[test]
    fn anonymous_fallback_projection_suppression_and_early_pressure() {
        let anonymous = mesh("invalid", "[]", "[]");
        let lower_valid = mesh(LOW, r#""bad""#, "[]");
        let source = format!("[{anonymous},{lower_valid}]");
        assert!(matches!(
            scan_meshes(raw(&source), |_| Ok::<_, ()>(())),
            Err(CollectionScanError::Structural {
                id: Some(id),
                field: MeshField::Positions,
                ..
            }) if id[15] == 1
        ));

        let valid = mesh(HIGH, "[]", "[]");
        let high_bad = mesh(HIGH, r#""bad""#, "[]");
        let source = format!("[{valid},{high_bad},{valid},{lower_valid},{valid}]");
        let mut calls = 0;
        assert!(matches!(
            scan_meshes(raw(&source), |_| {
                calls += 1;
                Ok::<_, ()>(())
            }),
            Err(CollectionScanError::Structural {
                id: Some(id),
                field: MeshField::Positions,
                ..
            }) if id[15] == 1
        ));
        assert_eq!(calls, 1);

        let source = format!("[{valid},{lower_valid}]");
        let mut calls = 0;
        assert!(matches!(
            scan_meshes(raw(&source), |_| {
                calls += 1;
                Err("pressure")
            }),
            Err(CollectionScanError::Projection {
                cause: "pressure",
                ..
            })
        ));
        assert_eq!(calls, 1);
    }

    #[test]
    fn camera_collection_uses_uuid_order_and_refuses_nonarray() {
        let low_bad = camera(LOW, r#""bad""#);
        let high_bad = camera(HIGH, r#""bad""#);
        for source in [
            format!("[{high_bad},{low_bad}]"),
            format!("[{low_bad},{high_bad}]"),
        ] {
            assert!(matches!(
                scan_cameras(raw(&source), |_| Ok::<_, ()>(())),
                Err(CollectionScanError::Structural {
                    id: Some(id),
                    field: CameraField::Position,
                    ..
                }) if id[15] == 1
            ));
        }
        assert!(matches!(
            scan_cameras(raw("null"), |_| Ok::<_, ()>(())),
            Err(CollectionScanError::Json(_))
        ));
    }
}
