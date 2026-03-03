fn main() {
    println!("cargo:rerun-if-changed=src");
    rust_sitter_tool::build_parser("src/grammar/mod.rs");
}
