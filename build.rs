//! build.rs — Ferrum asset pipeline.

fn main() {
    println!("cargo:rerun-if-changed=assets/");
    println!("cargo:rerun-if-changed=ferrum.toml");

    // TODO: implement shader validation, font bundling, typed asset handle generation
}
