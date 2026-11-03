//! Object-scoped strict decoding for the inactive shared render source.
//!
//! F3 and F4 will use one decoder with different projections. Mesh scratch
//! lives only through its callback; F3 retains at most its compact extrema
//! and identity, while F4 may copy the geometry into a product preparation.
//! Operation-wide charged allocation and diagnostics are separate work.

use serde_json::value::RawValue;

use crate::render_bound::transformed_box_within_world;
use crate::render_geometry_exact::{
    NonFinitePosition, TriangleSourceError, position_within_local_radius, triangle_is_nondegenerate,
};
use crate::render_json::{SourceText, parse_raw_object, uuid_bytes};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CameraFacts {
    pub(crate) id: [u8; 16],
    pub(crate) position_m: [f64; 3],
    pub(crate) orientation_xyzw: [f64; 4],
    pub(crate) vertical_fov_rad: f64,
    pub(crate) near_m: f64,
    pub(crate) far_m: f64,
    pub(crate) exposure: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum CameraField {
    Id,
    Position,
    Orientation,
    VerticalFov,
    Near,
    Far,
    Exposure,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum CameraProblem {
    Json(serde_json::Error),
    Missing,
    Duplicate,
    Unknown(String),
    InvalidId,
    Nonfinite,
    OutOfRange,
    InvalidOrientation,
}

#[derive(Debug)]
pub(crate) struct CameraDecodeError {
    pub(crate) id: Option<[u8; 16]>,
    pub(crate) field: CameraField,
    pub(crate) problem: CameraProblem,
}

const CAMERA_FIELDS: [&str; 7] = [
    "id",
    "position_m",
    "orientation_xyzw",
    "vertical_fov_rad",
    "near_m",
    "far_m",
    "exposure",
];
const CAMERA_FIELD_ORDER: [CameraField; 7] = [
    CameraField::Id,
    CameraField::Position,
    CameraField::Orientation,
    CameraField::VerticalFov,
    CameraField::Near,
    CameraField::Far,
    CameraField::Exposure,
];

fn camera_error(
    id: Option<[u8; 16]>,
    field: CameraField,
    problem: CameraProblem,
) -> CameraDecodeError {
    CameraDecodeError { id, field, problem }
}

/// Checks authored camera source meaning, independent of F4 target support.
/// The private exact transformed-box predicate supplies the same quaternion
/// band that instances use; a zero box/translation isolates that check.
pub(crate) fn decode_camera_v1(raw: &RawValue) -> Result<CameraFacts, CameraDecodeError> {
    let fields = parse_raw_object(raw.get(), &CAMERA_FIELDS)
        .map_err(|source| camera_error(None, CameraField::Id, CameraProblem::Json(source)))?;
    let decoded_id: Option<Result<SourceText<'_>, _>> =
        fields.values[0].map(|value| serde_json::from_str(value.get()));
    let possible_id = if fields.duplicate[0] {
        None
    } else {
        decoded_id
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .and_then(|text| uuid_bytes(&text.0))
    };
    if let Some(index) = fields.duplicate.iter().position(|&duplicate| duplicate) {
        return Err(camera_error(
            possible_id,
            CAMERA_FIELD_ORDER[index],
            CameraProblem::Duplicate,
        ));
    }
    let decoded_id =
        decoded_id.ok_or_else(|| camera_error(None, CameraField::Id, CameraProblem::Missing))?;
    let source_id = decoded_id
        .map_err(|source| camera_error(None, CameraField::Id, CameraProblem::Json(source)))?;
    let id = uuid_bytes(&source_id.0)
        .ok_or_else(|| camera_error(None, CameraField::Id, CameraProblem::InvalidId))?;

    let read = |index: usize, field| {
        fields.values[index].ok_or_else(|| camera_error(Some(id), field, CameraProblem::Missing))
    };
    let position_m: [f64; 3] = serde_json::from_str(read(1, CameraField::Position)?.get())
        .map_err(|source| {
            camera_error(Some(id), CameraField::Position, CameraProblem::Json(source))
        })?;
    if position_m.iter().any(|v| !v.is_finite()) {
        return Err(camera_error(
            Some(id),
            CameraField::Position,
            CameraProblem::Nonfinite,
        ));
    }
    if position_m.iter().any(|v| v.abs() > 1_000_000.0) {
        return Err(camera_error(
            Some(id),
            CameraField::Position,
            CameraProblem::OutOfRange,
        ));
    }
    let orientation_xyzw: [f64; 4] = serde_json::from_str(read(2, CameraField::Orientation)?.get())
        .map_err(|source| {
            camera_error(
                Some(id),
                CameraField::Orientation,
                CameraProblem::Json(source),
            )
        })?;
    if orientation_xyzw.iter().any(|v| !v.is_finite()) {
        return Err(camera_error(
            Some(id),
            CameraField::Orientation,
            CameraProblem::Nonfinite,
        ));
    }
    if !matches!(
        transformed_box_within_world([0.0; 3], [0.0; 3], [0.0; 3], orientation_xyzw, [1.0; 3]),
        Ok(true)
    ) {
        return Err(camera_error(
            Some(id),
            CameraField::Orientation,
            CameraProblem::InvalidOrientation,
        ));
    }
    let vertical_fov_rad: f64 = serde_json::from_str(read(3, CameraField::VerticalFov)?.get())
        .map_err(|source| {
            camera_error(
                Some(id),
                CameraField::VerticalFov,
                CameraProblem::Json(source),
            )
        })?;
    // The representable f64 PI is immediately below true pi; its successor
    // exceeds true pi. This is the exact decoded-f64 test for 0 < fov < pi.
    if !vertical_fov_rad.is_finite() {
        return Err(camera_error(
            Some(id),
            CameraField::VerticalFov,
            CameraProblem::Nonfinite,
        ));
    }
    if !(vertical_fov_rad > 0.0 && vertical_fov_rad <= std::f64::consts::PI) {
        return Err(camera_error(
            Some(id),
            CameraField::VerticalFov,
            CameraProblem::OutOfRange,
        ));
    }
    let near_m: f64 = serde_json::from_str(read(4, CameraField::Near)?.get())
        .map_err(|source| camera_error(Some(id), CameraField::Near, CameraProblem::Json(source)))?;
    if !near_m.is_finite() {
        return Err(camera_error(
            Some(id),
            CameraField::Near,
            CameraProblem::Nonfinite,
        ));
    }
    if near_m <= 0.0 {
        return Err(camera_error(
            Some(id),
            CameraField::Near,
            CameraProblem::OutOfRange,
        ));
    }
    let far_m: f64 = serde_json::from_str(read(5, CameraField::Far)?.get())
        .map_err(|source| camera_error(Some(id), CameraField::Far, CameraProblem::Json(source)))?;
    if !far_m.is_finite() {
        return Err(camera_error(
            Some(id),
            CameraField::Far,
            CameraProblem::Nonfinite,
        ));
    }
    if far_m <= near_m {
        return Err(camera_error(
            Some(id),
            CameraField::Far,
            CameraProblem::OutOfRange,
        ));
    }
    let exposure: f64 =
        serde_json::from_str(read(6, CameraField::Exposure)?.get()).map_err(|source| {
            camera_error(Some(id), CameraField::Exposure, CameraProblem::Json(source))
        })?;
    if !exposure.is_finite() {
        return Err(camera_error(
            Some(id),
            CameraField::Exposure,
            CameraProblem::Nonfinite,
        ));
    }
    if exposure <= 0.0 {
        return Err(camera_error(
            Some(id),
            CameraField::Exposure,
            CameraProblem::OutOfRange,
        ));
    }
    if let Some(unknown) = fields.unknown {
        return Err(camera_error(
            Some(id),
            CameraField::Unknown,
            CameraProblem::Unknown(unknown.0.into_owned()),
        ));
    }
    Ok(CameraFacts {
        id,
        position_m,
        orientation_xyzw,
        vertical_fov_rad,
        near_m,
        far_m,
        exposure,
    })
}

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
    if let Some(unknown) = fields.unknown {
        return Err(structural(
            Some(id),
            MeshField::Unknown,
            MeshProblem::Unknown(unknown.0.into_owned()),
        ));
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
    const CAMERA: &str = r#"{"id":"a6b9c300-0000-4000-8000-000000000013","position_m":[0,0,5],"orientation_xyzw":[0,0,0,1],"vertical_fov_rad":1.0471975511965976,"near_m":0.1,"far_m":20,"exposure":0.0001}"#;

    fn raw(text: &str) -> &RawValue {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn camera_source_uses_exact_quaternion_and_closed_f64_pi_predecessor() {
        let camera = decode_camera_v1(raw(CAMERA)).unwrap();
        assert_eq!(camera.id[15], 19);
        assert_eq!(camera.position_m, [0.0, 0.0, 5.0]);
        let at_pi = CAMERA.replace("1.0471975511965976", "3.141592653589793");
        assert!(decode_camera_v1(raw(&at_pi)).is_ok());
        let above_pi = CAMERA.replace("1.0471975511965976", "3.1415926535897936");
        assert!(matches!(
            decode_camera_v1(raw(&above_pi)),
            Err(CameraDecodeError {
                field: CameraField::VerticalFov,
                problem: CameraProblem::OutOfRange,
                ..
            })
        ));
        let invalid_q = CAMERA.replace("[0,0,0,1]", "[0,0,0,2]");
        assert!(matches!(
            decode_camera_v1(raw(&invalid_q)),
            Err(CameraDecodeError {
                field: CameraField::Orientation,
                problem: CameraProblem::InvalidOrientation,
                ..
            })
        ));
        let invalid_position = CAMERA.replace("[0,0,5]", "[0,0,1000000.0000000001]");
        assert!(matches!(
            decode_camera_v1(raw(&invalid_position)),
            Err(CameraDecodeError {
                field: CameraField::Position,
                problem: CameraProblem::OutOfRange,
                ..
            })
        ));
    }

    #[test]
    fn camera_diagnostics_use_schema_order_and_anonymous_duplicate_id() {
        let id = "a6b9c300-0000-4000-8000-000000000013";
        let reordered = format!(
            r#"{{"exposure":0.0001,"far_m":20,"near_m":0.1,"vertical_fov_rad":1,"orientation_xyzw":[0,0,0,1],"position_m":[0,0,5],"id":"{id}"}}"#
        );
        assert_eq!(decode_camera_v1(raw(&reordered)).unwrap().id[15], 19);
        let bad_fields = reordered
            .replace("[0,0,5]", "\"bad\"")
            .replace("0.0001", "0");
        assert!(matches!(
            decode_camera_v1(raw(&bad_fields)),
            Err(CameraDecodeError { id: Some(found), field: CameraField::Position, problem: CameraProblem::Json(_), .. }) if found[15] == 19
        ));
        let unknown_and_bad = bad_fields.replacen('{', "{\"z_extra\":true,", 1);
        assert!(matches!(
            decode_camera_v1(raw(&unknown_and_bad)),
            Err(CameraDecodeError {
                field: CameraField::Position,
                problem: CameraProblem::Json(_),
                ..
            })
        ));
        let unknown_only = reordered.replacen('{', "{\"z_extra\":true,", 1);
        assert!(matches!(
            decode_camera_v1(raw(&unknown_only)),
            Err(CameraDecodeError {
                field: CameraField::Unknown,
                problem: CameraProblem::Unknown(name),
                ..
            }) if name == "z_extra"
        ));
        for ids in [
            format!(r#""id":"{id}","id":"invalid""#),
            format!(r#""id":"invalid","i\u0064":"{id}""#),
        ] {
            let object = format!("{{{ids}}}");
            assert!(matches!(
                decode_camera_v1(raw(&object)),
                Err(CameraDecodeError {
                    id: None,
                    field: CameraField::Id,
                    problem: CameraProblem::Duplicate
                })
            ));
        }
        let positional =
            r#"["a6b9c300-0000-4000-8000-000000000013",[0,0,5],[0,0,0,1],1,0.1,20,0.0001]"#;
        assert!(matches!(
            decode_camera_v1(raw(positional)),
            Err(CameraDecodeError {
                problem: CameraProblem::Json(_),
                ..
            })
        ));
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
        let unknown_and_bad = format!(
            r#"{{"id":"{id}","z_extra":true,"positions_m":"wrong","uv0":[],"triangles":[]}}"#
        );
        assert!(matches!(
            decode_mesh_v1(raw(&unknown_and_bad), |_| Ok::<_, ()>(())),
            Err(MeshDecodeError::Structural {
                field: MeshField::Positions,
                problem: MeshProblem::Json(_),
                ..
            })
        ));
        let unknown_only = MESH.replacen('{', "{\"z_extra\":true,", 1);
        assert!(matches!(
            decode_mesh_v1(raw(&unknown_only), |_| Ok::<_, ()>(())),
            Err(MeshDecodeError::Structural {
                field: MeshField::Unknown,
                problem: MeshProblem::Unknown(name),
                ..
            }) if name == "z_extra"
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
