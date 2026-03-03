fn main() {
    println!("cargo:rerun-if-changed=src");
    rust_sitter_tool::build_parser("src/grammar/mod.rs");

    // Generate TextMate grammar from the same source of truth.
    let textmate = rust_sitter_tool::TextMateBuilder::default()
        .scope_name("kide")
        .build("src/grammar/mod.rs")
        .expect("failed to generate TextMate grammar");

    let out_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vscode-kide/syntaxes/kide.tmLanguage.json");
    let json = serde_json::to_string_pretty(&textmate).expect("failed to serialize TextMate JSON");
    std::fs::write(&out_path, format!("{json}\n")).expect("failed to write TextMate grammar");
}
