//! Immutable chart samples. A frame selects an interval from the displayed map;
//! the shader mixes each corner before bilinear sampling and joining.
use super::write_layer;
use crate::WaterError;
use thrum_water_model::Map;

pub(super) struct ChartLayers {
    river: usize,
    chart: usize,
    first: u32,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "prepared sample rows and weights are read in GPU metres"
)]
impl ChartLayers {
    pub(super) fn prepare(
        map: &Map,
        body: usize,
        layers: &mut u32,
        limit: u32,
    ) -> Result<Option<Self>, WaterError> {
        let Some((river, chart)) = map.river_of(body) else {
            return Ok(None);
        };
        let count = map.river(river).expect("prepared river").gauges.len() - 1;
        if count == 0 {
            // A single sample already occupies the ordinary chart layer.
            return Ok(None);
        }
        let first = *layers;
        let count = u32::try_from(count)
            .ok()
            .and_then(|count| count.checked_add(first))
            .filter(|&end| end <= limit)
            .ok_or(WaterError::UnsupportedMap(
                "river samples exceed map layer limit",
            ))?;
        *layers = count;
        Ok(Some(Self {
            river,
            chart,
            first,
        }))
    }

    pub(super) fn write(
        &self,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        map: &Map,
        pixels: &mut [[f32; 4]],
    ) {
        let river = map.river(self.river).expect("prepared river");
        let chart = &river.charts[self.chart];
        for (index, pair) in chart.samples.windows(2).enumerate() {
            let (a, b) = (&pair[0], &pair[1]);
            let (ga, gb) = (river.gauges[index], river.gauges[index + 1]);
            for (i, pixel) in pixels.iter_mut().enumerate() {
                let bed = map.bed().heights()[i];
                let wetting = if a.levels[i] <= bed && b.levels[i] > bed {
                    ((chart.wetting[index][i].unwrap_or(gb) - ga) / (gb - ga)) as f32
                } else {
                    -1.0
                };
                *pixel = [
                    b.levels[i] as f32,
                    b.currents[i][0] as f32,
                    b.currents[i][1] as f32,
                    wetting,
                ];
            }
            write_layer(queue, texture, self.first + index as u32, pixels);
        }
    }

    pub(super) fn interval(&self, map: &Map) -> (u32, f32) {
        let gauges = &map.river(self.river).expect("prepared river").gauges;
        let last = gauges.len() - 1;
        let gauge = map.gauge(self.river).clamp(gauges[0], gauges[last]);
        let lower = gauges[..last].partition_point(|&sample| sample <= gauge) - 1;
        let weight = (gauge - gauges[lower]) / (gauges[lower + 1] - gauges[lower]);
        (self.first + lower as u32, weight as f32)
    }

    pub(super) fn first(&self) -> u32 {
        self.first
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "admitted sample layers and GPU chart values"
)]
pub(super) fn write_flow(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    map: &Map,
    body: usize,
    first: u32,
    pixels: &mut [[f32; 4]],
) {
    let (river, chart) = map.river_of(body).expect("river flow layers");
    let chart = &map.river(river).expect("prepared river").charts[chart];
    for (index, sample) in chart.samples.iter().enumerate() {
        for (i, pixel) in pixels.iter_mut().enumerate() {
            *pixel = [
                sample.gains[i][0] as f32,
                sample.gains[i][1] as f32,
                sample.loss[i] as f32,
                0.0,
            ];
        }
        write_layer(queue, texture, first + index as u32, pixels);
    }
}
