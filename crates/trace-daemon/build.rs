fn main() {
    // rust-embed reads the Vite output at compile time. Tell Cargo to rebuild
    // the daemon when web assets change, even if no Rust source file changed.
    println!("cargo:rerun-if-changed=../../apps/web/dist");
}
