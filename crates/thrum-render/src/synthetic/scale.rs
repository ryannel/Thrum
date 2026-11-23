//! Prepared render subrectangles within an output-sized target.

/// A render scale supported by portable temporal reconstruction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RenderScale {
    /// Draw at output resolution.
    #[default]
    Native,
    /// Draw at 85 percent of output resolution.
    Percent85,
    /// Draw at 75 percent of output resolution.
    Percent75,
    /// Draw at 67 percent of output resolution.
    Percent67,
    /// Draw at half output resolution.
    Half,
}

impl RenderScale {
    /// Supported steps in descending order.
    pub const STEPS: [Self; 5] = [
        Self::Native,
        Self::Percent85,
        Self::Percent75,
        Self::Percent67,
        Self::Half,
    ];

    /// Nominal ratio as an integer percentage.
    pub const fn percent(self) -> u32 {
        match self {
            Self::Native => 100,
            Self::Percent85 => 85,
            Self::Percent75 => 75,
            Self::Percent67 => 67,
            Self::Half => 50,
        }
    }

    /// Pixel extent of the top-left drawn subrectangle.
    pub fn extent(self, output: [u32; 2]) -> [u32; 2] {
        let percent = self.percent();
        output.map(|axis| {
            let whole = axis / 100 * percent;
            let remainder = (axis % 100 * percent + 50) / 100;
            (whole + remainder).max(1)
        })
    }
}
