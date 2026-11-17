/// A native control panel could not be built or captured.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelError {
    /// Schedule construction on AppKit's main thread.
    MainThreadRequired,
    /// The panel no longer has a drawable content view.
    MissingContent,
    /// AppKit could not allocate a bitmap for the panel.
    BitmapUnavailable,
    /// AppKit could not encode the panel's bitmap as PNG.
    PngUnavailable,
}

impl std::fmt::Display for PanelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MainThreadRequired => "the control panel must be built on AppKit's main thread",
            Self::MissingContent => "the panel has no content view",
            Self::BitmapUnavailable => "AppKit could not allocate a bitmap for the panel",
            Self::PngUnavailable => "AppKit could not encode the panel as PNG",
        })
    }
}
impl std::error::Error for PanelError {}
