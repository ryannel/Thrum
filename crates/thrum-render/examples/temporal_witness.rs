//! Emit the renderer's temporal witness for the browser qualification bundle.

use std::io::{self, Write};

fn main() -> io::Result<()> {
    io::stdout()
        .lock()
        .write_all(thrum_render::synthetic::TEMPORAL_WITNESS_JSON.as_bytes())
}
