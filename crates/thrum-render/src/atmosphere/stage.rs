//! What each atmosphere stage reads and writes. The stage's bind group and its
//! graph accesses are both built from [`AtmosphereStage::chart_reads`], so the
//! barrier the graph emits covers exactly the charts the shader samples.

use crate::pass_key::AtmosphereStage;
use crate::shader_constants::{ATMOSPHERE_EXTENTS, ATMOSPHERE_PLANES};

/// One persistent atmosphere chart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Chart {
    /// Transmittance to the atmosphere's top.
    Transmittance,
    /// Multiple-scattering transfer.
    MultipleScattering,
    /// Camera sky view.
    SkyView,
    /// Ground irradiance.
    Ground,
}

impl Chart {
    // Position in the generated shader extent tables: transmittance, multiple
    // scattering, sky view, ground.
    const fn table_row(self) -> usize {
        match self {
            Self::Transmittance => 0,
            Self::MultipleScattering => 1,
            Self::SkyView => 2,
            Self::Ground => 3,
        }
    }

    /// Texel width and height of one plane.
    pub(crate) const fn extent(self) -> [u32; 2] {
        ATMOSPHERE_EXTENTS[self.table_row()]
    }

    /// Planes stacked vertically in the chart's texture.
    pub(crate) const fn planes(self) -> u32 {
        ATMOSPHERE_PLANES[self.table_row()]
    }

    /// Owner offset of the chart's persistent graph resource.
    pub(crate) const fn owner_offset(self) -> u64 {
        self.table_row() as u64
    }
}

/// One value per chart, reached by the chart's name.
#[derive(Clone, Debug)]
pub(crate) struct Charts<T> {
    /// Transmittance.
    pub(crate) transmittance: T,
    /// Multiple scattering.
    pub(crate) multiple_scattering: T,
    /// Sky view.
    pub(crate) sky_view: T,
    /// Ground irradiance.
    pub(crate) ground: T,
}

impl<T> Charts<T> {
    /// Build one value per chart: transmittance, multiple scattering, sky view, ground.
    pub(crate) fn try_from_fn<E>(mut make: impl FnMut(Chart) -> Result<T, E>) -> Result<Self, E> {
        Ok(Self {
            transmittance: make(Chart::Transmittance)?,
            multiple_scattering: make(Chart::MultipleScattering)?,
            sky_view: make(Chart::SkyView)?,
            ground: make(Chart::Ground)?,
        })
    }

    /// Build one value per chart: transmittance, multiple scattering, sky view, ground.
    pub(crate) fn from_fn(mut make: impl FnMut(Chart) -> T) -> Self {
        Self::try_from_fn::<std::convert::Infallible>(|chart| Ok(make(chart)))
            .unwrap_or_else(|never| match never {})
    }

    /// The value for `chart`.
    pub(crate) const fn get(&self, chart: Chart) -> &T {
        match chart {
            Chart::Transmittance => &self.transmittance,
            Chart::MultipleScattering => &self.multiple_scattering,
            Chart::SkyView => &self.sky_view,
            Chart::Ground => &self.ground,
        }
    }

    /// Map each value, keeping its chart.
    pub(crate) fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> Charts<U> {
        Charts::from_fn(|chart| f(self.get(chart)))
    }
}

/// A chart a stage samples, and the binding its shader samples it at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChartRead {
    /// The chart sampled.
    pub(crate) chart: Chart,
    /// Its binding in the stage's group.
    pub(crate) binding: u32,
}

const fn read(chart: Chart, binding: u32) -> ChartRead {
    ChartRead { chart, binding }
}

// Each atmosphere cache bit, one per group of stages that refresh together.
pub(super) const TRANSPORT_VALID: u8 = 1;
pub(super) const SKY_VALID: u8 = 2;
pub(super) const DIFFUSE_VALID: u8 = 4;
pub(super) const AERIAL_VALID: u8 = 8;
pub(super) const GROUND_VALID: u8 = 16;
pub(super) const REFLECTION_VALID: u8 = 32;
pub(super) const DFG_VALID: u8 = 64;

impl AtmosphereStage {
    /// The charts this stage samples. Its bind group and its graph accesses are
    /// both built from this one table.
    pub(crate) const fn chart_reads(self) -> &'static [ChartRead] {
        const TRANSMITTANCE: ChartRead = read(Chart::Transmittance, 1);
        const MULTIPLE: ChartRead = read(Chart::MultipleScattering, 2);
        const GROUND: ChartRead = read(Chart::Ground, 5);
        const SKY: ChartRead = read(Chart::SkyView, 1);
        match self {
            Self::Transmittance | Self::Dfg => &[],
            Self::MultipleScattering => &[TRANSMITTANCE],
            Self::Ground | Self::Aerial => &[TRANSMITTANCE, MULTIPLE],
            Self::SkyView => &[TRANSMITTANCE, MULTIPLE, GROUND],
            Self::Diffuse | Self::Reflection(_) => &[SKY],
        }
    }

    /// The chart this stage integrates, for the four chart stages.
    pub(crate) const fn chart_output(self) -> Option<Chart> {
        match self {
            Self::Transmittance => Some(Chart::Transmittance),
            Self::MultipleScattering => Some(Chart::MultipleScattering),
            Self::SkyView => Some(Chart::SkyView),
            Self::Ground => Some(Chart::Ground),
            Self::Diffuse | Self::Aerial | Self::Dfg | Self::Reflection(_) => None,
        }
    }

    /// The cache bit that, while set, lets the frame skip this stage.
    pub(crate) const fn validity(self) -> u8 {
        match self {
            Self::Transmittance | Self::MultipleScattering => TRANSPORT_VALID,
            Self::SkyView => SKY_VALID,
            Self::Diffuse => DIFFUSE_VALID,
            Self::Aerial => AERIAL_VALID,
            Self::Ground => GROUND_VALID,
            Self::Dfg => DFG_VALID,
            Self::Reflection(_) => REFLECTION_VALID,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChartRead;
    use crate::pass_key::AtmosphereStage;

    #[test]
    fn every_stage_reads_each_chart_once_at_its_own_binding() {
        for stage in AtmosphereStage::ORDER {
            let reads: &[ChartRead] = stage.chart_reads();
            for (index, read) in reads.iter().enumerate() {
                assert!(
                    reads[index + 1..]
                        .iter()
                        .all(|other| other.chart != read.chart && other.binding != read.binding),
                    "{stage:?} binds a chart or a binding twice"
                );
                assert_ne!(read.binding, 0, "binding 0 is the uniform");
            }
            assert!(
                stage
                    .chart_output()
                    .is_none_or(|output| reads.iter().all(|read| read.chart != output)),
                "{stage:?} reads the chart it writes"
            );
        }
    }
}
