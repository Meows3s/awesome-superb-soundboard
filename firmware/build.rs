//! Build script: make `memory.x` visible to the linker.
//!
//! `cortex-m-rt` looks for a linker script named `memory.x` on the linker
//! search path; we copy ours next to the compiled artifacts so flip-link can
//! find it via `-Tmemory.x`.
fn main() {
    println!("cargo:rerun-if-changed=memory.x");
    println!(
        "cargo:rustc-link-search={}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap()
    );
}
