//! Browser-only diagnostic of actual supported and CPU-overflow light lists.
use thrum_render::synthetic::{
    FrameLighting, LightListProbe, OffscreenTarget, PreparedSynthetic, SyntheticLight,
    SyntheticPointLight, SyntheticScene,
};
use wasm_bindgen::prelude::*;

fn render_error(error: impl std::fmt::Debug) -> JsValue {
    JsValue::from_str(&format!("{error:?}"))
}

/// One submitted read-only light-list observation; poll it from the browser.
#[wasm_bindgen]
pub struct BrowserLightProbe {
    device: wgpu::Device,
    probe: LightListProbe,
    count: u32,
    ties: bool,
    gpu: bool,
    dropped: usize,
    preparation_ms: f64,
}

/// Prepare/submit a 4, 32 or 64 source all-overlap fixture on actual WebGPU.
/// # Errors
/// Returns invalid fixture, GPU preparation or diagnostic encoding failures.
#[wasm_bindgen]
pub async fn create_light_list_probe(count: u32, ties: bool) -> Result<BrowserLightProbe, JsValue> {
    if ![4, 32, 64].contains(&count) {
        return Err(super::js_error("count must be 4, 32 or 64"));
    }
    let (_instance, device, queue, caps) = super::effect_diagnostic_device().await?;
    let mut scene = SyntheticScene::grid(1).map_err(render_error)?;
    let source_count = u16::try_from(count).map_err(super::js_error)?;
    scene.local_lights = (0..source_count)
        .map(|i| SyntheticPointLight {
            position: [0.0, 0.0, 0.5],
            radius: 128.0,
            candela: [if ties { 1.0 } else { f32::from(i + 1) }; 3],
        })
        .collect();
    let start = js_sys::Date::now();
    let prepared = PreparedSynthetic::prepare_with_capabilities(
        &caps,
        &queue,
        &scene,
        FrameLighting::Diagnostic(SyntheticLight::default()),
        1.0,
    )
    .await
    .map_err(render_error)?;
    let preparation_ms = js_sys::Date::now() - start;
    let target = OffscreenTarget::new(&device, &prepared, 32, 32).map_err(render_error)?;
    let mut probe = LightListProbe::new(&prepared, &scene).map_err(render_error)?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, 1, &target, None)
        .map_err(render_error)?;
    probe.encode(&mut encoder).map_err(render_error)?;
    queue.submit([encoder.finish()]);
    Ok(BrowserLightProbe {
        device,
        probe,
        count,
        ties,
        gpu: prepared.gpu_clustered(),
        dropped: prepared.dropped_light_references(),
        preparation_ms,
    })
}

#[wasm_bindgen]
impl BrowserLightProbe {
    /// Return the exact readback result when mapping completes.
    /// # Errors
    /// Returns an asynchronous mapping failure.
    pub fn poll(&mut self) -> Result<Option<String>, JsValue> {
        let _ = self.device.poll(wgpu::PollType::Poll);
        self.probe.try_collect().map_err(render_error).map(|result|result.map(|equal|
            serde_json::json!({"sources":self.count,"ties":self.ties,"equal":equal,
                "gpu_build":self.gpu,"cpu_build":!self.gpu,"clusters":9216,
                "gpu_list_executions":u32::from(self.gpu),"cpu_list_preparations":u32::from(!self.gpu),
                "checked_bytes":self.probe.checked_bytes(),"dropped_references":self.dropped,
                "preparation_wall_ms":self.preparation_ms,
                "scope":"full actual head/index-buffer readback against existing CPU oracle; preparation includes validation/allocation, not an isolated CPU timer"}).to_string()))
    }
}
