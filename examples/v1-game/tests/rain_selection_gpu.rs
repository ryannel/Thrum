//! The same cooked exit scene with default, explicit off and rain selected.

#[cfg(target_os = "macos")]
mod rain_support;

#[cfg(target_os = "macos")]
mod metal {
    use std::time::Duration;

    use super::rain_support as rain;

    use thrum_render::runtime::FrameWorld;
    use v1_game::{GameContent, GameSetup, RainSelection, exit_authority};

    #[test]
    fn exit_keeps_its_authority_graph_and_pixels_until_rain_is_selected() {
        let Some(gpu) = rain::open("rain selection on Metal") else {
            return;
        };
        let store = rain::Store::cook();
        pollster::block_on(async {
            assert_eq!(GameSetup::default().rain, RainSelection::Off);
            let setup = GameSetup {
                content: GameContent::Exit,
                ..GameSetup::default()
            };
            let mut baseline = store.host(&gpu, setup).await;
            let mut off = store
                .host(
                    &gpu,
                    GameSetup {
                        rain: RainSelection::Off,
                        ..setup
                    },
                )
                .await;
            let mut original = exit_authority().expect("ordinary exit authority");
            let original_page = original.published_page().expect("ordinary endpoint");
            let page = off
                .world_mut()
                .publication()
                .expect("exit publication")
                .page;
            assert_eq!(page.len(), original_page.len());
            assert_eq!(page.slot_capacity(), original_page.slot_capacity());
            assert!(page.rain().is_none());
            let visits = |page: &v1_game::simulation::PublishedPage| {
                let mut rows = Vec::new();
                page.visit(|handle, id, pos, material| rows.push((handle, id, pos, material)));
                rows
            };
            assert_eq!(visits(&page), visits(&original_page));
            assert!(
                !off.scene_shader_source()
                    .expect("ordinary shader")
                    .contains("fn ground_wetness(")
            );
            let passes = |host: &rain::Host| {
                host.scene_graph()
                    .expect("exit graph")
                    .pass_names()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            };
            assert_eq!(passes(&baseline), passes(&off));
            assert!(
                !passes(&off)
                    .iter()
                    .any(|name| name.contains("water") || name.contains("rain"))
            );
            assert_eq!(baseline.storage_report(), off.storage_report());
            assert_eq!(
                baseline.capture_pixels().expect("default frame"),
                off.capture_pixels().expect("rain off frame")
            );

            let mut on = store
                .host(
                    &gpu,
                    GameSetup {
                        rain: RainSelection::Soaked,
                        ..setup
                    },
                )
                .await;
            let page = on.world_mut().publication().expect("rain publication").page;
            assert_eq!(page.len(), original_page.len());
            assert!(page.rain().expect("committed rain input").flux > 0.0);
            assert!(
                on.scene_shader_source()
                    .expect("wet shader")
                    .contains("fn ground_wetness(")
            );
            assert!(passes(&on).iter().any(|name| name.contains("water")));
            on.drive(Duration::ZERO, None)
                .expect("submit selected valley");
            let soaked = on.source().rain_observation().expect("rain observation");
            assert!(
                soaked["wet_depths_m"]
                    .as_array()
                    .expect("two puddles")
                    .iter()
                    .all(|depth| depth.as_f64().expect("depth in metres") > 0.01),
                "{soaked}"
            );
        });
    }
}
