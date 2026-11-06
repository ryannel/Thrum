/// Product family; one logical asset has exactly one kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProductKind {
    /// Meshlet mesh container.
    Mesh,
    /// Two-dimensional KTX2 texture with a full mip chain.
    Texture,
    /// Three-dimensional KTX2 texture, such as baked cloud density.
    Volume,
}

impl ProductKind {
    /// Manifest code.
    pub fn code(self) -> u8 {
        match self {
            Self::Mesh => 1,
            Self::Texture => 2,
            Self::Volume => 3,
        }
    }

    /// Decode a manifest code.
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Mesh),
            2 => Some(Self::Texture),
            3 => Some(Self::Volume),
            _ => None,
        }
    }
}

/// Target profile a product was cooked for. The running device picks its preferred profile from
/// its capability record and falls back to [`Profile::Portable`], which every device can load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Profile {
    /// Uncompressed RGBA8/R8 textures and volumes; meshes are profile-independent and use this.
    Portable,
    /// BC7 textures and BC4 sliced-3D volumes (desktop GPUs and Apple Silicon Macs).
    DesktopBc,
    /// ASTC 4×4 textures with R8 volumes (Apple GPUs and Android tile GPUs).
    AppleAstc,
}

impl Profile {
    /// Every profile, in code order.
    pub const ALL: [Self; 3] = [Self::Portable, Self::DesktopBc, Self::AppleAstc];

    /// Manifest code.
    pub fn code(self) -> u8 {
        match self {
            Self::Portable => 0,
            Self::DesktopBc => 1,
            Self::AppleAstc => 2,
        }
    }

    /// Decode a manifest code.
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|profile| profile.code() == code)
    }

    /// Stable lower-case name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::DesktopBc => "desktop-bc",
            Self::AppleAstc => "apple-astc",
        }
    }

    /// Parse [`Profile::name`].
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|profile| profile.name() == name)
    }
}
