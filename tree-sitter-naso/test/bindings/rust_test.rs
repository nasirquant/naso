//! Integration tests for tree-sitter-naso Rust bindings
//!
//! These tests verify that the generated parser correctly handles
//! all Naso language constructs.

use tree_sitter_naso::{language, util};
use tree_sitter::Parser;

fn parse(source: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    parser.set_language(language()).expect("Failed to set language");
    parser.parse(source, None).expect("Failed to parse")
}

#[test]
fn test_comprehensive_parsing() {
    let source = include_str!("../../corpus/comprehensive.naso");
    let tree = parse(source);
    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(tree.root_node().named_child_count() > 0);
}

#[test]
fn test_quantum_parsing() {
    let source = include_str!("../../corpus/quantum.naso");
    let tree = parse(source);
    assert_eq!(tree.root_node().kind(), "source_file");

    let intrinsics = util::find_quantum_intrinsics(&tree);
    // qalloc x4, hadamard x3, cnot x2, measure x4, qfree x4, bell_pair x1, qft x1, grover_oracle x1
    assert!(intrinsics.len() >= 15, "Expected at least 15 quantum intrinsics, found {}", intrinsics.len());
}

#[test]
fn test_tensor_parsing() {
    let source = include_str!("../../corpus/tensor.naso");
    let tree = parse(source);
    assert_eq!(tree.root_node().kind(), "source_file");

    let tensors = util::find_tensor_operations(&tree);
    assert!(tensors.len() >= 10, "Expected at least 10 tensor operations, found {}", tensors.len());
}

#[test]
fn test_polyhedral_parsing() {
    let source = include_str!("../../corpus/polyhedral.naso");
    let tree = parse(source);
    assert_eq!(tree.root_node().kind(), "source_file");

    let loops = util::find_polyhedral_loops(&tree);
    assert!(loops.len() >= 15, "Expected at least 15 polyhedral loops, found {}", loops.len());
}

#[test]
fn test_quantity_annotations() {
    let source = r#"
fn linear(x: [1] i32) -> [1] i32 { x }
fn proof(x: [0] i32) { }
fn bounded(x: [N] i32) { }
fn heap(x: [*] i32) { }
fn nested(x: [1] [N] i32) { }
"#;
    let tree = parse(source);
    let root = tree.root_node();

    // Find all quantity_annotation nodes
    let mut count = 0;
    let mut cursor = root.walk();
    visit_quantity_annotations(&mut cursor, &mut count);
    assert_eq!(count, 5);
}

fn visit_quantity_annotations(cursor: &mut tree_sitter::TreeCursor, count: &mut usize) {
    let node = cursor.node();
    if node.kind() == "quantity_annotation" {
        *count += 1;
    }
    if cursor.goto_first_child() {
        loop {
            visit_quantity_annotations(cursor, count);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_inout_parameters() {
    let source = r#"
fn mutate(inout x: i32) { }
fn swap(inout a: i32, inout b: i32) { }
fn linear_mutate(inout x: [1] i32) { }
"#;
    let tree = parse(source);
    let root = tree.root_node();

    // Check that inout parameters are parsed
    let mut inout_count = 0;
    let mut cursor = root.walk();
    visit_inout_params(&mut cursor, &mut inout_count);
    assert_eq!(inout_count, 3);
}

fn visit_inout_params(cursor: &mut tree_sitter::TreeCursor, count: &mut usize) {
    let node = cursor.node();
    if node.kind() == "parameter" {
        // Check if first child is "inout"
        if let Some(first) = node.named_child(0) {
            if first.kind() == "inout" || 
               (first.kind() == "identifier" && first.utf8_text(node.tree().text().as_bytes()).unwrap() == "inout") {
                *count += 1;
            }
        }
    }
    if cursor.goto_first_child() {
        loop {
            visit_inout_params(cursor, count);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_uncomputation_statement() {
    let source = r#"
fn test() {
    let q = qalloc();
    uncompute q;
}
"#;
    let tree = parse(source);
    let root = tree.root_node();

    let mut uncompute_count = 0;
    let mut cursor = root.walk();
    visit_uncompute(&mut cursor, &mut uncompute_count);
    assert_eq!(uncompute_count, 1);
}

fn visit_uncompute(cursor: &mut tree_sitter::TreeCursor, count: &mut usize) {
    let node = cursor.node();
    if node.kind() == "uncomputation_statement" {
        *count += 1;
    }
    if cursor.goto_first_child() {
        loop {
            visit_uncompute(cursor, count);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_defer_statement() {
    let source = r#"
fn test() {
    let x = acquire();
    defer {
        release(x);
    }
}
"#;
    let tree = parse(source);
    let root = tree.root_node();

    let mut defer_count = 0;
    let mut cursor = root.walk();
    visit_defer(&mut cursor, &mut defer_count);
    assert_eq!(defer_count, 1);
}

fn visit_defer(cursor: &mut tree_sitter::TreeCursor, count: &mut usize) {
    let node = cursor.node();
    if node.kind() == "defer_statement" {
        *count += 1;
    }
    if cursor.goto_first_child() {
        loop {
            visit_defer(cursor, count);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_module_and_imports() {
    let source = r#"
module std {
    pub fn foo() { }
}

import { foo } from "std";
import bar from "baz";
"#;
    let tree = parse(source);
    let root = tree.root_node();

    let mut module_count = 0;
    let mut import_count = 0;
    let mut cursor = root.walk();
    visit_modules_imports(&mut cursor, &mut module_count, &mut import_count);

    assert_eq!(module_count, 1);
    assert_eq!(import_count, 2);
}

fn visit_modules_imports(cursor: &mut tree_sitter::TreeCursor, modules: &mut usize, imports: &mut usize) {
    let node = cursor.node();
    if node.kind() == "module_declaration" {
        *modules += 1;
    }
    if node.kind() == "import_declaration" {
        *imports += 1;
    }
    if cursor.goto_first_child() {
        loop {
            visit_modules_imports(cursor, modules, imports);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_structs_enums() {
    let source = r#"
struct Point { x: f64, y: f64 }
struct Generic<T> { value: T }

enum Option<T> { Some(T), None }
enum Result<T, E> { Ok(T), Err(E) }
"#;
    let tree = parse(source);
    let root = tree.root_node();

    let mut struct_count = 0;
    let mut enum_count = 0;
    let mut cursor = root.walk();
    visit_structs_enums(&mut cursor, &mut struct_count, &mut enum_count);

    assert_eq!(struct_count, 2);
    assert_eq!(enum_count, 2);
}

fn visit_structs_enums(cursor: &mut tree_sitter::TreeCursor, structs: &mut usize, enums: &mut usize) {
    let node = cursor.node();
    if node.kind() == "struct_declaration" {
        *structs += 1;
    }
    if node.kind() == "enum_declaration" {
        *enums += 1;
    }
    if cursor.goto_first_child() {
        loop {
            visit_structs_enums(cursor, structs, enums);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_generic_functions() {
    let source = r#"
fn identity<T>(x: T) -> T { x }
fn compose<A, B, C>(f: fn(A) -> B, g: fn(B) -> C, x: A) -> C { g(f(x)) }
fn constrained<T: Clone + Debug>(x: T) -> T { x }
"#;
    let tree = parse(source);
    let root = tree.root_node();

    let mut fn_count = 0;
    let mut cursor = root.walk();
    visit_functions(&mut cursor, &mut fn_count);
    assert_eq!(fn_count, 3);
}

fn visit_functions(cursor: &mut tree_sitter::TreeCursor, count: &mut usize) {
    let node = cursor.node();
    if node.kind() == "function_declaration" {
        *count += 1;
    }
    if cursor.goto_first_child() {
        loop {
            visit_functions(cursor, count);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_error_recovery() {
    // Test that parser recovers from errors and continues parsing
    let source = r#"
fn valid1() { }
fn invalid( { }  // Missing parameter name and type
fn valid2() { }
"#;
    let tree = parse(source);
    let root = tree.root_node();

    // Should have error nodes but still parse valid functions
    let mut error_count = 0;
    let mut valid_fn_count = 0;
    let mut cursor = root.walk();
    visit_errors_and_fns(&mut cursor, &mut error_count, &mut valid_fn_count);

    assert!(error_count > 0, "Should have error nodes for recovery");
    assert_eq!(valid_fn_count, 2, "Should parse both valid functions");
}

fn visit_errors_and_fns(cursor: &mut tree_sitter::TreeCursor, errors: &mut usize, fns: &mut usize) {
    let node = cursor.node();
    if node.is_error() || node.is_missing() {
        *errors += 1;
    }
    if node.kind() == "function_declaration" {
        *fns += 1;
    }
    if cursor.goto_first_child() {
        loop {
            visit_errors_and_fns(cursor, errors, fns);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_round_trip_parsing() {
    let source = r#"
fn test() {
    let x: [1] i32 = 42;
    let y = x;
    qalloc();
    hadamard(x);
}
"#;
    let mut parser = Parser::new();
    parser.set_language(language()).unwrap();

    // Parse once
    let tree1 = parser.parse(source, None).unwrap();
    let cst1 = tree1.root_node().to_sexp();

    // Parse again
    let tree2 = parser.parse(source, None).unwrap();
    let cst2 = tree2.root_node().to_sexp();

    // CSTs should be identical
    assert_eq!(cst1, cst2);
}

#[test]
fn test_quantum_with_quantity() {
    let source = r#"
fn linear_quantum(q: [1] qubit) -> [1] qubit {
    hadamard(q);
    q
}

fn proof_quantum(q: [0] qubit) {
    // Proof-only qubit, erased at runtime
}
"#;
    let tree = parse(source);
    let root = tree.root_node();

    let mut linear_quantum = 0;
    let mut proof_quantum = 0;
    let mut cursor = root.walk();
    visit_quantum_qty(&mut cursor, &mut linear_quantum, &mut proof_quantum);

    assert_eq!(linear_quantum, 1);
    assert_eq!(proof_quantum, 1);
}

fn visit_quantum_qty(cursor: &mut tree_sitter::TreeCursor, linear: &mut usize, proof: &mut usize) {
    let node = cursor.node();
    if node.kind() == "parameter" {
        // Check for [1] qubit
        for child in node.named_children(&mut node.walk()) {
            if child.kind() == "quantity_annotation" {
                let text = child.utf8_text(node.tree().text().as_bytes()).unwrap_or("");
                if text.contains("[1]") && text.contains("qubit") {
                    *linear += 1;
                }
                if text.contains("[0]") && text.contains("qubit") {
                    *proof += 1;
                }
            }
        }
    }
    if cursor.goto_first_child() {
        loop {
            visit_quantum_qty(cursor, linear, proof);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[test]
fn test_math_expressions() {
    let source = r#"
fn math() {
    let a = 1 + 2 * 3;
    let b = (x + y) / z;
    let c = -x;
    let d = !flag;
    let e = x as f64;
}
"#;
    let tree = parse(source);
    assert_eq!(tree.root_node().kind(), "source_file");
    // Should parse without errors
    let mut has_error = false;
    let mut cursor = tree.root_node().walk();
    check_errors(&mut cursor, &mut has_error);
    assert!(!has_error, "Should not have parse errors");
}

fn check_errors(cursor: &mut tree_sitter::TreeCursor, has_error: &mut bool) {
    let node = cursor.node();
    if node.is_error() || node.is_missing() {
        *has_error = true;
    }
    if cursor.goto_first_child() {
        loop {
            check_errors(cursor, has_error);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}