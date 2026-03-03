fn main() {
    println!("cargo:rerun-if-changed=src");
    rust_sitter_tool::build_parser("src/grammar/mod.rs");

    // Generate TextMate grammar from the same source of truth.
    let mut textmate = rust_sitter_tool::TextMateBuilder::default()
        .scope_name("kide")
        .build("src/grammar/mod.rs")
        .expect("failed to generate TextMate grammar");

    // Post-process: remove overly greedy catch-all patterns (e.g. `[^{}]+` from
    // BlockFragment) that would swallow keywords and strings inside blocks.
    if let Some(repo) = textmate.get_mut("repository") {
        if let Some(idents) = repo.get_mut("identifiers") {
            if let Some(patterns) = idents.get_mut("patterns") {
                if let Some(arr) = patterns.as_array_mut() {
                    arr.retain(|p| {
                        p.get("match")
                            .and_then(|m| m.as_str())
                            .map_or(true, |m| !m.starts_with("[^"))
                    });
                }
            }
        }
    }

    let out_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vscode-kide/syntaxes/kide.tmLanguage.json");
    let json = serde_json::to_string_pretty(&textmate).expect("failed to serialize TextMate JSON");
    std::fs::write(&out_path, format!("{json}\n")).expect("failed to write TextMate grammar");
}
