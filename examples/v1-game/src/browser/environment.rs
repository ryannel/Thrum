//! Explicit environment qualification contribution, installed only on request.
use super::BrowserGame;
use std::{cell::RefCell, rc::Rc};
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface},
};
use wasm_bindgen::prelude::*;

struct EnvironmentContribution {
    library: Rc<RefCell<String>>,
    products: Option<EnvironmentProducts>,
}

impl SceneContribution for EnvironmentContribution {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: false,
            compute: false,
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let products = surface
            .environment_products()
            .ok_or("scene has no atmosphere")?;
        self.library
            .borrow_mut()
            .clone_from(&products.shader_source);
        self.products = Some(products);
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
    fn computes(&self, _: &ContributionFrame) -> bool {
        false
    }
    fn encode_compute(&mut self, _: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {}
    fn encode_after_compute(&mut self, _: &mut wgpu::CommandEncoder) {}
    fn draw(&self, _: &mut wgpu::RenderPass<'_>) {}
    fn submitted(&mut self, _: bool) {}
}

#[wasm_bindgen]
impl BrowserGame {
    /// Install a read-only qualification contribution and return its lent WGSL.
    /// The browser referee observes the public WebGPU layout/group creation.
    /// # Errors
    /// Refuses loss or a scene without atmosphere products.
    pub fn environment_library(&mut self) -> Result<String, JsValue> {
        self.shell.poll_completed().map_err(super::js_error)?;
        let library = Rc::new(RefCell::new(String::new()));
        self.shell
            .set_scene_contribution(Box::new(EnvironmentContribution {
                library: Rc::clone(&library),
                products: None,
            }))
            .map_err(super::js_error)?;
        let source = library.borrow().clone();
        Ok(source)
    }
}
