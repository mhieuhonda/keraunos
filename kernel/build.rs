//! Build script: wires the per-architecture linker script into the link step.
//!
//! Each architecture keeps its linker script at `kernel/arch/<arch>/linker.ld`.
//! Architectures without a script yet (aarch64, riscv64 scaffolds) link with
//! the toolchain defaults and are only type-checked (`cargo check`) today.

use std::{env, path::PathBuf};

fn main() {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("cargo sets CARGO_CFG_TARGET_ARCH");
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let script = manifest_dir.join("arch").join(&arch).join("linker.ld");

    if script.exists() {
        println!("cargo:rustc-link-arg=-T{}", script.display());
        println!("cargo:rerun-if-changed={}", script.display());
    }
}
