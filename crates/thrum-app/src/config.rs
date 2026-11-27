/// The game's window settings, worker name and diagnostic labels.
#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "macos"), derive(Copy))]
pub struct Config {
    /// Prefix of every standard-error diagnostic, for example the binary name.
    pub name: &'static str,
    /// Environment variable bounding the run's presented frames (at most 100 000). When it is
    /// unset, [`crate::run`] presents 600 frames and [`crate::run_until_closed`] runs until the
    /// window closes.
    pub frames_variable: &'static str,
    /// Window title until the first frame profile replaces it.
    pub window_title: &'static str,
    /// Leading text of the live status and profiler titles.
    pub title: &'static str,
    /// Name of the render owner thread.
    pub render_thread: &'static str,
    /// Title of the demo-control panel.
    pub panel_title: &'static str,
    /// Initial window inner size in physical pixels; a capture renders at this size too.
    pub window_size: [u32; 2],
    /// Initial Mac window position in logical screen coordinates, when specified.
    pub window_position: Option<[i32; 2]>,
    /// Keep the Mac window above normal windows for the lifetime of the run.
    pub window_always_on_top: bool,
    /// Mac time reserved before the display target for advancing and drawing.
    /// When unset, the render owner starts as soon as the frame is ready.
    pub frame_budget: Option<std::time::Duration>,
    /// Build-authored Metal libraries and preparation mode for initial setup and recovery.
    #[cfg(target_os = "macos")]
    pub metal_preparation: Option<thrum_render::capabilities::metallib::Preparation>,
    /// Override the Mac display-link request for this launch.
    #[cfg(target_os = "macos")]
    pub mac_frame_rate: Option<MacFrameRate>,
    /// Force a prepared output scale for a 3D Mac scene.
    #[cfg(target_os = "macos")]
    pub mac_output_scale: Option<thrum_render::synthetic::RenderScale>,
}

impl Config {
    /// Text for a game called `name` (the diagnostic prefix, usually the binary name) whose
    /// window and panel are titled `title`: a 1600×900 window, `THRUM_FRAMES` to bound a
    /// scripted run and a render thread named `thrum-render`. Change any field afterwards.
    #[must_use]
    pub const fn new(name: &'static str, title: &'static str) -> Self {
        Self {
            name,
            frames_variable: "THRUM_FRAMES",
            window_title: title,
            title,
            render_thread: "thrum-render",
            panel_title: title,
            window_size: [1600, 900],
            window_position: None,
            window_always_on_top: false,
            frame_budget: None,
            #[cfg(target_os = "macos")]
            metal_preparation: None,
            #[cfg(target_os = "macos")]
            mac_frame_rate: None,
            #[cfg(target_os = "macos")]
            mac_output_scale: None,
        }
    }
}
/// A diagnostic override for the Mac display-link request.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug)]
pub enum MacFrameRate {
    /// Request a fixed 60 Hz.
    Sixty,
    /// Request a fixed 120 Hz.
    Full,
}
