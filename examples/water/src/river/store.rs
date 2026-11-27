use std::sync::atomic::{AtomicU64, Ordering};
use std::{error::Error, fs, path::Path, time::Instant};

use thrum_river_model::{Declaration, Grid, Manning, Profile, Receiver, Source, Spring};
use thrum_water_model::{Bed, ReachSample, RiverFile, UpperSample};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn grid(bed: &Bed, start: [usize; 2], size: [usize; 2]) -> Result<Grid, Box<dyn Error>> {
    let (_, spacing, [width, _]) = bed.lattice();
    let origin = bed.point(start[0] + start[1] * width);
    let heights = (start[1]..start[1] + size[1])
        .flat_map(|z| {
            let first = z * width + start[0];
            bed.heights()[first..first + size[0]].iter().copied()
        })
        .collect();
    Ok(Grid::new(origin, spacing, size, heights)?)
}

pub(super) fn load(
    bed: &Bed,
    directory: &Path,
    fall: bool,
) -> Result<[RiverFile; 2], Box<dyn Error>> {
    let stream = grid(bed, [188, 46], [25, 48])?; // x 150..210, z -110..7.5
    let outlet = grid(bed, [198, 76], [85, 33])?; // x 175..385, z -35..45
    let profile = Profile {
        threads: 1,
        join_length: 8.0,
        margin_cells: 2,
        ..Profile::default()
    };
    let spring = Declaration {
        name: super::NAMES[0].into(),
        springs: vec![Spring {
            name: "Spring".into(),
            point: super::SPRING,
            discharge: 5.0,
            radius: 3.5,
        }],
        sources: vec![],
        receivers: vec![Receiver {
            name: "Lake".into(),
            seed: [182.5, 0.0],
            extent: [[150.0, -20.0], [210.0, 7.5]],
            level: 12.0,
        }],
        manning: Manning::Uniform(0.045),
    };
    let lake = Declaration {
        name: super::NAMES[1].into(),
        springs: vec![],
        sources: vec![Source {
            name: "Lake".into(),
            seed: [182.5, 5.0],
            extent: [[175.0, -35.0], [209.0, 45.0]],
            level: 12.0,
            range: [12.0, 13.0],
        }],
        receivers: vec![Receiver {
            name: "Sea".into(),
            seed: [372.5, 5.0],
            extent: [[347.0, -35.0], [385.0, 45.0]],
            level: -0.5,
        }],
        manning: Manning::Uniform(0.045),
    };
    let stream_flows = if fall {
        &[0.0, 0.02, 3.0, 10.0][..]
    } else {
        &[5.0, 7.5, 10.0]
    };
    let stream_samples = stream_flows
        .iter()
        .map(|&flow| vec![flow])
        .collect::<Vec<_>>();
    let stream_profile = if fall {
        Profile {
            level_rate: 8e-5,
            imbalance: 0.025,
            step_budget: 80_000,
            ..profile.clone()
        }
    } else {
        profile.clone()
    };
    Ok([
        load_one(
            directory,
            &stream,
            &spring,
            &stream_profile,
            &stream_samples,
        )?,
        load_one(
            directory,
            &outlet,
            &lake,
            &Profile {
                level_rate: 0.006,
                imbalance: 0.004,
                ..profile
            },
            &[],
        )?,
    ])
}

#[expect(
    clippy::print_stderr,
    reason = "native preparation reports each cold solve"
)]
fn load_one(
    directory: &Path,
    grid: &Grid,
    declaration: &Declaration,
    profile: &Profile,
    springs: &[Vec<f64>],
) -> Result<RiverFile, Box<dyn Error>> {
    let key = thrum_river_model::key_for_samples(grid, declaration, profile, springs);
    let key_hex: String = super::key_hex(&key);
    let path = directory.join(format!("{key_hex}.river"));
    match fs::read(&path) {
        Ok(bytes) => match RiverFile::read(&bytes, &key) {
            Ok(file) => return Ok(file),
            Err(error) => eprintln!("rebuilding {}: {error:?}", path.display()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let start = Instant::now();
    let closed_spring = springs.first().is_some_and(|sample| sample == &[0.0]);
    let running = if closed_spring {
        &springs[1..]
    } else {
        springs
    };
    let samples = thrum_river_model::solve_samples(grid, declaration, profile, running)?;
    for (gauge, solution) in samples.gauges.iter().zip(&samples.solutions) {
        let report = &solution.report;
        eprintln!(
            "river sample {} gauge={gauge:.8} seconds={:.6} steps={} level_rate={:.8} imbalance={:.8}",
            declaration.name, report.seconds, report.steps, report.level_rate, report.imbalance,
        );
    }
    let mut file = samples.file(key)?;
    if closed_spring {
        add_closed_spring_sample(&mut file, grid)?;
    }
    let bytes = thrum_river_model::write(&file);
    fs::create_dir_all(directory)?;
    let worker = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("{}-{worker}.tmp", std::process::id()));
    if let Err(error) = fs::write(&temporary, &bytes).and_then(|()| fs::rename(&temporary, &path)) {
        drop(fs::remove_file(temporary));
        return Err(error.into());
    }
    eprintln!(
        "solved {} ({} samples in {:.3} s, {} bytes): {}",
        declaration.name,
        file.upper.len() + 1,
        start.elapsed().as_secs_f64(),
        bytes.len(),
        path.display(),
    );
    Ok(file)
}

// The zero-inflow limit is hydrostatic. The spring perch is dry and the
// lower receiving body still owns its own standing water.
fn add_closed_spring_sample(file: &mut RiverFile, grid: &Grid) -> Result<(), Box<dyn Error>> {
    let source = grid
        .cell(super::SPRING)
        .ok_or("spring outside the fall grid")?;
    let closed = grid.heights()[source].min(file.gauge_level - 0.001);
    let wetting_gauge = f64::midpoint(closed, file.gauge_level);
    for reach in &mut file.reaches {
        let cells = reach.size[0] * reach.size[1];
        let bed_at = |cell: usize| {
            let ix = u32::try_from(cell % reach.size[0]).expect("bed column");
            let iz = u32::try_from(cell / reach.size[0]).expect("bed row");
            let point = [
                f64::from(ix).mul_add(reach.spacing, reach.origin[0]),
                f64::from(iz).mul_add(reach.spacing, reach.origin[1]),
            ];
            grid.cell(point)
                .map(|index| grid.heights()[index])
                .ok_or("reach outside the fall grid")
        };
        let bed = (0..cells).map(bed_at).collect::<Result<Vec<_>, _>>()?;
        let wetting = reach
            .levels
            .iter()
            .zip(&bed)
            .map(|(&level, &bed)| (level > bed).then_some(wetting_gauge))
            .collect();
        let dry = reach
            .levels
            .iter()
            .zip(&bed)
            .map(|(&level, &bed)| level.min(bed - reach.dry_margin))
            .collect();
        let sample = ReachSample {
            levels: std::mem::replace(&mut reach.levels, dry),
            currents: std::mem::replace(&mut reach.currents, vec![[0.0; 2]; cells]),
            loss: std::mem::replace(&mut reach.loss, vec![0.0; cells]),
            gains: std::mem::replace(&mut reach.gains, vec![[0.0; 2]; cells]),
            wetting,
        };
        reach.upper.insert(0, sample);
    }
    let falls = std::mem::take(&mut file.falls);
    let lip_wetting = falls
        .iter()
        .flat_map(|fall| &fall.segments)
        .map(|segment| (segment.id, wetting_gauge))
        .collect();
    file.upper.insert(
        0,
        UpperSample {
            gauge_level: std::mem::replace(&mut file.gauge_level, closed),
            falls,
            lip_wetting,
        },
    );
    Ok(())
}
