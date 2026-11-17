//! Compiles the UI, gives the exe its icon and version details, and in the release build builds
//! the bridge to carry inside the exe.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[path = "build/resources.rs"]
mod resources;

type Failure = Box<dyn std::error::Error>;

fn main() -> Result<(), Failure> {
    // The fonts go into the exe as they are: nothing is read from the source folder when it runs.
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/app.slint", config)?;

    // Load the exe's imported system DLLs from System32 only.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo::rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    }
    resources::link_exe_resources(Path::new(&std::env::var("OUT_DIR")?))?;

    println!("cargo::rustc-check-cfg=cfg(carries_bridge)");
    // The release build carries the bridge, built from the same source, so Sobreiro.exe stays
    // one file. A debug build does not: the tests, clippy and dev runs stay quick, and the
    // bridge is found beside the exe, as 1.1's dev builds found it.
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        carry_bridge()?;
    }
    Ok(())
}

/// Builds `sobreiro-bridge.exe` in a folder of its own and tells the compiler where it is.
fn carry_bridge() -> Result<(), Failure> {
    let cargo = std::env::var("CARGO")?;
    let target = std::env::var("TARGET")?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let workspace = Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).join("../..");
    let target_dir = out_dir.join("bridge");

    let status = Command::new(cargo)
        .args(["build", "--release", "--locked"])
        .args(["--package", "sobreiro-bridge", "--bin", "sobreiro-bridge"])
        .arg("--manifest-path")
        .arg(workspace.join("Cargo.toml"))
        .args(["--target", &target])
        .arg("--target-dir")
        .arg(&target_dir)
        // A clippy run does not lint inside the bridge's build.
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("CLIPPY_ARGS")
        // What this script prints is cargo's to read: the build's own output goes to stderr.
        .stdout(Stdio::from(std::io::stderr()))
        .status()?;
    if !status.success() {
        return Err("the bridge did not build".into());
    }

    let exe = target_dir
        .join(&target)
        .join("release")
        .join("sobreiro-bridge.exe");
    if !exe.is_file() {
        return Err("the bridge did not build: its exe is not where it should be".into());
    }
    println!("cargo::rustc-env=SOBREIRO_BRIDGE_EXE={}", exe.display());
    println!("cargo::rustc-cfg=carries_bridge");

    for path in [
        "Cargo.toml",
        "Cargo.lock",
        ".cargo/config.toml",
        "bridge",
        "core",
    ] {
        println!("cargo::rerun-if-changed={}", workspace.join(path).display());
    }
    Ok(())
}
