//! Declares the `frb_expand` cfg used by `flutter_rust_bridge` macros.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(frb_expand)");
}
