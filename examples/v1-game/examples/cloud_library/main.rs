//! Cook the V1 game's accepted cloud library and derive the headless footprints.

mod authoring;

use std::{error::Error, io, path::PathBuf};
use thrum_asset::Profile;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let (Some(work), Some(store), None) = (args.next(), args.next(), args.next()) else {
        return Err(
            io::Error::other("usage: cook_cloud_library <tvol-source-dir> <cooked-store>").into(),
        );
    };
    let library = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/clouds");
    authoring::cook(&library, &work, &store, Profile::ALL.to_vec())?;
    Ok(())
}
