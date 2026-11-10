//! Compile the macOS qualification signpost bridge with the platform SDK macros.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/app/qualification_signpost.c")
            .warnings_into_errors(true)
            .compile("thrum_mac_qualification_signpost");
    }
}
