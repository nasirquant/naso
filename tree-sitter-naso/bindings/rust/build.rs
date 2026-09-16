use std::path::PathBuf;

fn main() {
    let parser_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/parser.c");
    let scanner_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/scanner.c");

    cc::Build::new()
        .file(&parser_path)
        .file(&scanner_path)
        .include("../../src")
        .compile("tree-sitter-naso");

    println!("cargo:rerun-if-changed=../../src/parser.c");
    println!("cargo:rerun-if-changed=../../src/scanner.c");
    println!("cargo:rerun-if-changed=../../src/parser.h");
}