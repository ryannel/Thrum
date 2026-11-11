/// Text the shell shows on the game's behalf: window titles, diagnostics and thread names.
#[derive(Clone, Copy, Debug)]
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
        }
    }
}
