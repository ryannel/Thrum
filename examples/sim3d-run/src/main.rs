//! Plain executable used on the Mac and through simctl spawn.

use std::io::{self, Write};

fn main() -> io::Result<()> {
    io::stdout()
        .lock()
        .write_all(sim3d_run::vector_rows().as_bytes())
}
