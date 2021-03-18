use anyhow::Result;

use crate::{
    not_bash::{fs2, rm_rf, run},
    project_root,
};

pub struct ClientOpts {
    pub version: String,
    pub release_tag: String,
}

pub fn run_dist() -> Result<()> {
    let dist = project_root().join("dist");
    rm_rf(&dist)?;
    fs2::create_dir_all(&dist)?;

    dist_clarinet()?;
    Ok(())
}

fn dist_clarinet() -> Result<()> {
    if cfg!(target_os = "linux") {
        std::env::set_var("CC", "clang");
        run!(
            "cargo build --manifest-path ./Cargo.toml --bin clarinet --release
             --target x86_64-unknown-linux-musl
            "
            // We'd want to add, but that requires setting the right linker somehow
            // --features=jemalloc
        )?;
        run!("strip ./target/x86_64-unknown-linux-musl/release/clarinet")?;
    } else {
        run!("cargo build --manifest-path ./Cargo.toml --bin clarinet --release")?;
    }

    let (src, dst) = if cfg!(target_os = "linux") {
        ("./target/x86_64-unknown-linux-musl/release/clarinet", "./dist/clarinet-linux")
    } else if cfg!(target_os = "windows") {
        ("./target/release/clarinet.exe", "./dist/clarinet-windows.exe")
    } else if cfg!(target_os = "macos") {
        ("./target/release/clarinet", "./dist/clarinet-mac")
    } else {
        panic!("Unsupported OS")
    };

    fs2::copy(src, dst)?;

    Ok(())
}
