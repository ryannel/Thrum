use std::sync::atomic::{AtomicU64, Ordering};
use std::{error::Error, fs, path::Path, time::Instant};

use thrum_river_model::{Declaration, Grid, Manning, Profile, Receiver, Source, Spring};
use thrum_water_model::{Bed, RiverFile};

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

pub(super) fn load(bed: &Bed, directory: &Path) -> Result<[RiverFile; 2], Box<dyn Error>> {
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
    Ok([
        load_one(
            directory,
            &stream,
            &spring,
            &profile,
            &[vec![5.0], vec![7.5], vec![10.0]],
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
    let samples = thrum_river_model::solve_samples(grid, declaration, profile, springs)?;
    for (gauge, solution) in samples.gauges.iter().zip(&samples.solutions) {
        let report = &solution.report;
        eprintln!(
            "river sample {} gauge={gauge:.8} seconds={:.6} steps={} level_rate={:.8} imbalance={:.8}",
            declaration.name, report.seconds, report.steps, report.level_rate, report.imbalance,
        );
    }
    let file = samples.file(key)?;
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
        samples.gauges.len(),
        start.elapsed().as_secs_f64(),
        bytes.len(),
        path.display(),
    );
    Ok(file)
}
