//! Inactive render-v1 structural root walk and provisional typed projection.
//!
//! This coordinates the complete local grammar in frozen collection order. It
//! neither admits source nor checks global identity, typed edge closure,
//! instance bounds or F4 target limits. Every projection remains provisional:
//! callers must discard it after any failure and qualify it in later phases.

use crate::render_json::{
    CheckedRenderRecord, RenderEnvelopeError, check_render_record, render_envelope_slices,
};
use crate::render_source_collections::{
    CameraScanError, InstanceScanError, LocalLightScanError, MaterialScanError, MeshScanError,
    TextureScanError, scan_cameras, scan_instances, scan_local_lights, scan_materials, scan_meshes,
    scan_textures,
};
use crate::render_source_light::LocalLightFacts;
use crate::render_source_material::MaterialFacts;
use crate::render_source_objects::{CameraFacts, InstanceFacts, MeshEvent};
use crate::render_source_texture::CheckedTexture;

/// The closed render-v1 event vocabulary. One operational failure type covers
/// every callback, so allocation/storage refusal cannot become a source error.
/// Mesh slices are ephemeral; checked textures may borrow the original input.
/// Callbacks follow cameras, meshes, instances, materials, textures, then local
/// lights, with source array order inside each collection. Root identity and
/// the active-camera edge are returned only after all six collections pass;
/// no callback receives them as an accepted or globally resolved identity.
pub(crate) trait RenderRootProjection<'input> {
    type Error;

    fn camera(&mut self, facts: CameraFacts) -> Result<(), Self::Error>;
    fn mesh(&mut self, event: MeshEvent<'_>) -> Result<(), Self::Error>;
    fn instance(&mut self, facts: InstanceFacts) -> Result<(), Self::Error>;
    fn material(&mut self, facts: MaterialFacts) -> Result<(), Self::Error>;
    fn texture(&mut self, checked: CheckedTexture<'input>) -> Result<(), Self::Error>;
    fn local_light(&mut self, facts: LocalLightFacts) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RenderRootCounts {
    pub(crate) cameras: usize,
    pub(crate) meshes: usize,
    pub(crate) instances: usize,
    pub(crate) materials: usize,
    pub(crate) textures: usize,
    pub(crate) local_lights: usize,
}

/// Local structural facts only. This is neither an admission nor edge receipt:
/// even the active camera may be absent and object IDs may repeat globally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RenderRootSummary {
    pub(crate) id: [u8; 16],
    pub(crate) active_camera: [u8; 16],
    pub(crate) counts: RenderRootCounts,
}

#[derive(Debug)]
pub(crate) enum RenderRootError<E> {
    Envelope(RenderEnvelopeError),
    Cameras(CameraScanError<E>),
    Meshes(MeshScanError<E>),
    Instances(InstanceScanError<E>),
    Materials(MaterialScanError<E>),
    Textures(TextureScanError<E>),
    LocalLights(LocalLightScanError<E>),
}

/// Prewalk the whole input for bounded syntax/header validity before emitting
/// anything. Then finish each collection before moving to the next; its scanner
/// selects the canonical UUID/field/raw/ordinal source error. Operational
/// projection refusal aborts immediately. No collection-wide vector is built.
pub(crate) fn scan_render_root<'input, P: RenderRootProjection<'input>>(
    bytes: &'input [u8],
    projection: &mut P,
) -> Result<RenderRootSummary, RenderRootError<P::Error>> {
    let checked = check_render_record(bytes).map_err(RenderRootError::Envelope)?;
    scan_checked_render_root(checked, projection)
}

/// F3 dispatch can pass the opaque checked token from its existing header
/// walk, avoiding a second whole-record prewalk on the admission path.
pub(crate) fn scan_checked_render_root<'input, P: RenderRootProjection<'input>>(
    checked: CheckedRenderRecord<'input>,
    projection: &mut P,
) -> Result<RenderRootSummary, RenderRootError<P::Error>> {
    let root = render_envelope_slices(checked).map_err(RenderRootError::Envelope)?;
    let mut counts = RenderRootCounts::default();
    // Each callback consumes at least one byte of the input; these counts
    // cannot overflow usize within one addressable source slice.
    scan_cameras(root.cameras, |facts| {
        projection.camera(facts)?;
        counts.cameras += 1;
        Ok(())
    })
    .map_err(RenderRootError::Cameras)?;
    scan_meshes(root.meshes, |event| {
        projection.mesh(event)?;
        counts.meshes += 1;
        Ok(())
    })
    .map_err(RenderRootError::Meshes)?;
    scan_instances(root.instances, |facts| {
        projection.instance(facts)?;
        counts.instances += 1;
        Ok(())
    })
    .map_err(RenderRootError::Instances)?;
    scan_materials(root.materials, |facts| {
        projection.material(facts)?;
        counts.materials += 1;
        Ok(())
    })
    .map_err(RenderRootError::Materials)?;
    scan_textures(root.textures, |checked| {
        projection.texture(checked)?;
        counts.textures += 1;
        Ok(())
    })
    .map_err(RenderRootError::Textures)?;
    scan_local_lights(root.local_lights, |facts| {
        projection.local_light(facts)?;
        counts.local_lights += 1;
        Ok(())
    })
    .map_err(RenderRootError::LocalLights)?;
    Ok(RenderRootSummary {
        id: root.id,
        active_camera: root.active_camera,
        counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_source_collections::CollectionScanError;
    use crate::render_source_objects::{CameraField, CameraProblem, MeshField, MeshProblem};
    use crate::render_source_texture::emit_texture_chunks;

    const LOW: &str = "a6b9c300-0000-4000-8000-000000000001";
    const HIGH: &str = "a6b9c300-0000-4000-8000-000000000002";

    // Root field order deliberately differs from collection validation order.
    fn root(collections: [&str; 6]) -> String {
        let [cameras, meshes, instances, materials, textures, lights] = collections;
        format!(
            r#"{{"fields":{{"local_lights":{lights},"textures":{textures},"materials":{materials},"instances":{instances},"meshes":{meshes},"cameras":{cameras},"active_camera":"{LOW}"}},"schema":1,"type":"thrum.render.scene","id":"{HIGH}"}}"#
        )
    }

    fn camera(id: &str, position: &str) -> String {
        format!(
            r#"{{"id":"{id}","position_m":{position},"orientation_xyzw":[0,0,0,1],"vertical_fov_rad":1,"near_m":0.1,"far_m":20,"exposure":1}}"#
        )
    }

    fn mesh(id: &str, positions: &str, uv0: &str) -> String {
        format!(r#"{{"id":"{id}","positions_m":{positions},"uv0":{uv0},"triangles":[]}}"#)
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Pressure;

    #[derive(Default)]
    struct Projection<'input> {
        events: Vec<&'static str>,
        textures: Vec<CheckedTexture<'input>>,
        fail_at: Option<&'static str>,
        mesh_vertices: usize,
    }

    impl Projection<'_> {
        fn event(&mut self, name: &'static str) -> Result<(), Pressure> {
            self.events.push(name);
            if self.fail_at == Some(name) {
                Err(Pressure)
            } else {
                Ok(())
            }
        }
    }

    impl<'input> RenderRootProjection<'input> for Projection<'input> {
        type Error = Pressure;

        fn camera(&mut self, _: CameraFacts) -> Result<(), Self::Error> {
            self.event("camera")
        }

        fn mesh(&mut self, event: MeshEvent<'_>) -> Result<(), Self::Error> {
            self.mesh_vertices += event.positions_m.len();
            assert_eq!(event.positions_m.len(), event.uv0.len());
            self.event("mesh")
        }

        fn instance(&mut self, _: InstanceFacts) -> Result<(), Self::Error> {
            self.event("instance")
        }

        fn material(&mut self, _: MaterialFacts) -> Result<(), Self::Error> {
            self.event("material")
        }

        fn texture(&mut self, checked: CheckedTexture<'input>) -> Result<(), Self::Error> {
            self.event("texture")?;
            self.textures.push(checked);
            Ok(())
        }

        fn local_light(&mut self, _: LocalLightFacts) -> Result<(), Self::Error> {
            self.event("local_light")
        }
    }

    fn complete_root() -> String {
        let cameras = format!("[{}]", camera(LOW, "[0,0,0]"));
        let meshes = format!("[{}]", mesh(LOW, "[[0,0,0]]", "[[0,0]]"));
        let instances = format!(
            r#"[{{"id":"{LOW}","mesh":"{LOW}","material":"{LOW}","translation_m":[0,0,0],"orientation_xyzw":[0,0,0,1],"scale":[1,1,1]}}]"#
        );
        let materials = format!(
            r#"[{{"id":"{LOW}","alpha_mode":"opaque","two_sided":false,"sampler":{{"min_filter":"nearest","mag_filter":"nearest","address_u":"repeat","address_v":"repeat"}},"base_factor_rgba":[1,1,1,1],"base_texture":"{LOW}"}}]"#
        );
        let textures = format!(
            r#"[{{"id":"{LOW}","width":1,"height":1,"format":"rgba8-srgb","rgba8":"AAECAw=="}}]"#
        );
        let lights = format!(
            r#"[{{"id":"{LOW}","position_m":[0,0,0],"luminous_intensity_cd":0,"unit_luminance_linear_rec709":[1,1,1],"radius_m":0}}]"#
        );
        root([
            &cameras, &meshes, &instances, &materials, &textures, &lights,
        ])
    }

    #[test]
    fn complete_six_collection_root_retains_borrowed_texture_for_later_replay() {
        let source = complete_root();
        let mut projection = Projection::default();
        let summary = scan_render_root(source.as_bytes(), &mut projection).unwrap();
        assert_eq!(summary.id[15], 2);
        assert_eq!(summary.active_camera[15], 1);
        assert_eq!(
            summary.counts,
            RenderRootCounts {
                cameras: 1,
                meshes: 1,
                instances: 1,
                materials: 1,
                textures: 1,
                local_lights: 1,
            }
        );
        assert_eq!(
            projection.events,
            [
                "camera",
                "mesh",
                "instance",
                "material",
                "texture",
                "local_light"
            ]
        );
        assert_eq!(projection.mesh_vertices, 1);
        // Replay occurs after the complete coordinator has returned. The
        // retained checked token still borrows the original source string.
        let mut bytes = Vec::new();
        emit_texture_chunks(&projection.textures[0], |chunk| {
            bytes.extend_from_slice(chunk);
            Ok::<_, Pressure>(())
        })
        .unwrap();
        assert_eq!(bytes, [0, 1, 2, 3]);

        let checked = check_render_record(source.as_bytes()).unwrap();
        let mut from_dispatch = Projection::default();
        let dispatch_summary = scan_checked_render_root(checked, &mut from_dispatch).unwrap();
        assert_eq!(dispatch_summary, summary);
        assert_eq!(from_dispatch.events, projection.events);
    }

    #[test]
    fn archived_authored_render_record_passes_complete_local_structure() {
        // The byte-identical render member of the selected evidence package.
        // Its full scene/project edges and unique namespace remain later work.
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/evidence/render/R0-AUTHORED-RENDER-RECORD-2026-11-03.json"
        ));
        let mut projection = Projection::default();
        let summary = scan_render_root(bytes, &mut projection).unwrap();
        assert_eq!(
            summary.counts,
            RenderRootCounts {
                cameras: 1,
                meshes: 3,
                instances: 3,
                materials: 3,
                textures: 4,
                local_lights: 4,
            }
        );
        assert_eq!(projection.textures.len(), 4);
        for checked in &projection.textures {
            let mut decoded = 0;
            emit_texture_chunks(checked, |chunk| {
                decoded += chunk.len();
                Ok::<_, Pressure>(())
            })
            .unwrap();
            assert_eq!(decoded, checked.facts.decoded_len);
        }
    }

    #[test]
    fn local_summary_does_not_claim_active_camera_or_global_identity_closure() {
        let source = root(["[]"; 6]);
        let summary = scan_render_root(source.as_bytes(), &mut Projection::default()).unwrap();
        assert_eq!(summary.active_camera[15], 1);
        assert_eq!(summary.counts, RenderRootCounts::default());
        // The complete fixture above intentionally repeats LOW across all
        // object kinds; structural success does not establish uniqueness.
    }

    #[test]
    fn syntax_and_envelope_precede_all_provisional_events() {
        let source = complete_root();
        for broken in [
            format!("{source} trailing"),
            source.replace("\"local_lights\":[", "\"local_lights\":[{"),
            source.replace("\"active_camera\"", "\"unknown\""),
        ] {
            let mut projection = Projection::default();
            assert!(matches!(
                scan_render_root(broken.as_bytes(), &mut projection),
                Err(RenderRootError::Envelope(_))
            ));
            assert!(projection.events.is_empty());
        }
    }

    #[test]
    fn current_collection_finishes_canonical_errors_before_later_collections() {
        let low = camera(LOW, "[0,0]");
        let high = camera(HIGH, r#""bad""#);
        for cameras in [format!("[{high},{low}]"), format!("[{low},{high}]")] {
            let source = root([&cameras, "[{}]", "null", "null", "null", "null"]);
            let mut projection = Projection::default();
            assert!(matches!(
                scan_render_root(source.as_bytes(), &mut projection),
                Err(RenderRootError::Cameras(CollectionScanError::Structural {
                    id: Some(id),
                    field: CameraField::Position,
                    ..
                })) if id[15] == 1
            ));
            assert!(projection.events.is_empty());
        }
    }

    #[test]
    fn every_collection_keeps_frozen_priority_over_later_failures() {
        for first_failure in 0..6 {
            let mut collections = ["null"; 6];
            collections[..first_failure].fill("[]");
            let source = root(collections);
            let mut projection = Projection::default();
            let error = scan_render_root(source.as_bytes(), &mut projection).unwrap_err();
            let collection = match error {
                RenderRootError::Cameras(CollectionScanError::Json(_)) => 0,
                RenderRootError::Meshes(CollectionScanError::Json(_)) => 1,
                RenderRootError::Instances(CollectionScanError::Json(_)) => 2,
                RenderRootError::Materials(CollectionScanError::Json(_)) => 3,
                RenderRootError::Textures(CollectionScanError::Json(_)) => 4,
                RenderRootError::LocalLights(CollectionScanError::Json(_)) => 5,
                unexpected => panic!("unexpected failure: {unexpected:?}"),
            };
            assert_eq!(collection, first_failure);
            assert!(projection.events.is_empty());
        }
    }

    #[test]
    fn root_preserves_raw_tie_selection_and_original_array_ordinal() {
        let vertex_zero = mesh(LOW, "[[2000,0,0]]", "[[0,0]]");
        let vertex_one = mesh(LOW, "[[0,0,0],[2000,0,0]]", "[[0,0],[0,0]]");
        assert!(vertex_one < vertex_zero);
        for (meshes, expected_ordinal) in [
            (format!("[{vertex_zero},{vertex_one}]"), 1),
            (format!("[{vertex_one},{vertex_zero}]"), 0),
        ] {
            let source = root(["[]", &meshes, "null", "null", "null", "null"]);
            assert!(matches!(
                scan_render_root(source.as_bytes(), &mut Projection::default()),
                Err(RenderRootError::Meshes(CollectionScanError::Structural {
                    ordinal,
                    field: MeshField::Positions,
                    problem: MeshProblem::PositionOutsideLocalRadius { vertex: 1 },
                    ..
                })) if ordinal == expected_ordinal
            ));
        }
    }

    #[test]
    fn provisional_earlier_events_survive_failure_for_caller_to_discard() {
        let cameras = format!("[{}]", camera(LOW, "[0,0,0]"));
        let good_mesh = mesh(HIGH, "[]", "[]");
        let high_bad = mesh(HIGH, r#""bad""#, "[]");
        let low_bad = mesh(LOW, "[]", r#""bad""#);
        for bad in [
            format!("[{good_mesh},{high_bad},{good_mesh},{low_bad}]"),
            format!("[{good_mesh},{low_bad},{good_mesh},{high_bad}]"),
        ] {
            let source = root([&cameras, &bad, "null", "null", "null", "null"]);
            let mut projection = Projection::default();
            assert!(matches!(
                scan_render_root(source.as_bytes(), &mut projection),
                Err(RenderRootError::Meshes(CollectionScanError::Structural {
                    id: Some(id),
                    field: MeshField::Uv0,
                    problem: MeshProblem::Json(_),
                    ..
                })) if id[15] == 1
            ));
            assert_eq!(projection.events, ["camera", "mesh"]);
        }
    }

    #[test]
    fn operational_projection_refusal_aborts_without_becoming_source_error() {
        let source = complete_root();
        let mut projection = Projection {
            fail_at: Some("texture"),
            ..Projection::default()
        };
        assert!(matches!(
            scan_render_root(source.as_bytes(), &mut projection),
            Err(RenderRootError::Textures(CollectionScanError::Projection {
                ordinal: 0,
                cause: Pressure,
                ..
            }))
        ));
        assert_eq!(
            projection.events,
            ["camera", "mesh", "instance", "material", "texture"]
        );
        assert!(projection.textures.is_empty());

        let invalid_camera = camera(LOW, r#""bad""#);
        let cameras = format!("[{},{}]", camera(HIGH, "[0,0,0]"), invalid_camera);
        let source = root([&cameras, "null", "null", "null", "null", "null"]);
        let mut projection = Projection {
            fail_at: Some("camera"),
            ..Projection::default()
        };
        assert!(matches!(
            scan_render_root(source.as_bytes(), &mut projection),
            Err(RenderRootError::Cameras(CollectionScanError::Projection {
                cause: Pressure,
                ..
            }))
        ));
        assert_eq!(projection.events, ["camera"]);
    }

    #[test]
    fn collection_type_failure_keeps_collection_and_json_error() {
        let source = root(["[]", "[]", "null", "null", "null", "null"]);
        assert!(matches!(
            scan_render_root(source.as_bytes(), &mut Projection::default()),
            Err(RenderRootError::Instances(CollectionScanError::Json(_)))
        ));
        let invalid = camera(LOW, r#""bad""#);
        let source = root([&format!("[{invalid}]"), "[]", "[]", "[]", "[]", "[]"]);
        assert!(matches!(
            scan_render_root(source.as_bytes(), &mut Projection::default()),
            Err(RenderRootError::Cameras(CollectionScanError::Structural {
                problem: CameraProblem::Json(_),
                ..
            }))
        ));
    }
}
