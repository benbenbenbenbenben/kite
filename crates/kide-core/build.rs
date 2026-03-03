use std::path::PathBuf;

fn main() {
    compile_grammar(
        "tree-sitter-rust",
        PathBuf::from("../../grammars/rust/tree-sitter-rust-0.24.0/src"),
    );
    compile_grammar(
        "tree-sitter-typescript",
        PathBuf::from("../../grammars/typescript/tree-sitter-typescript-0.23.2/typescript/src"),
    );
    compile_grammar(
        "tree-sitter-tsx",
        PathBuf::from("../../grammars/typescript/tree-sitter-typescript-0.23.2/tsx/src"),
    );
    compile_grammar(
        "tree-sitter-prisma",
        PathBuf::from("../../grammars/prisma/tree-sitter-prisma-1.6.0/src"),
    );
}

fn compile_grammar(lib_name: &str, grammar_src: PathBuf) {
    println!("cargo:rerun-if-changed={}", grammar_src.display());
    let mut build = cc::Build::new();
    build
        .include(&grammar_src)
        .file(grammar_src.join("parser.c"))
        .warnings(false);
    let scanner_path = grammar_src.join("scanner.c");
    if scanner_path.exists() {
        build.file(scanner_path);
    }
    build.compile(lib_name);
}
