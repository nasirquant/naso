//! Tree-sitter bindings for the Naso programming language
//!
//! This crate provides Rust bindings for the Naso tree-sitter grammar,
//! enabling syntax highlighting, parsing, and CST manipulation in Rust.

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_naso() -> *const ();
}

/// Returns the Tree-sitter [`Language`] for Naso.
///
/// # Example
///
/// ```rust
/// use tree_sitter::Parser;
///
/// let mut parser = Parser::new();
/// parser.set_language(tree_sitter_naso::language()).unwrap();
/// let tree = parser.parse("fn main() { qalloc(); }", None).unwrap();
/// ```
pub fn language() -> Language {
    unsafe { Language::from_raw(tree_sitter_naso()) }
}

/// The node types present in the Naso grammar.
pub mod node_kinds {
    pub const SOURCE_FILE: &str = "source_file";
    pub const MODULE_DECLARATION: &str = "module_declaration";
    pub const IMPORT_DECLARATION: &str = "import_declaration";
    pub const FUNCTION_DECLARATION: &str = "function_declaration";
    pub const STRUCT_DECLARATION: &str = "struct_declaration";
    pub const ENUM_DECLARATION: &str = "enum_declaration";
    pub const TYPE_ALIAS_DECLARATION: &str = "type_alias_declaration";
    pub const CONST_DECLARATION: &str = "const_declaration";
    pub const STATIC_DECLARATION: &str = "static_declaration";
    pub const QUANTITY_ANNOTATION: &str = "quantity_annotation";
    pub const FUNCTION_SIGNATURE: &str = "function_signature";
    pub const PARAMETER: &str = "parameter";
    pub const GENERIC_PARAMETERS: &str = "generic_parameters";
    pub const FORALL_LOOP: &str = "forall_loop";
    pub const TILE_LOOP: &str = "tile_loop";
    pub const FUSE_LOOP: &str = "fuse_loop";
    pub const QALLOC_CALL: &str = "qalloc_call";
    pub const QFREE_CALL: &str = "qfree_call";
    pub const HADAMARD_CALL: &str = "hadamard_call";
    pub const CNOT_CALL: &str = "cnot_call";
    pub const MEASURE_CALL: &str = "measure_call";
    pub const BELL_PAIR_CALL: &str = "bell_pair_call";
    pub const QFT_CALL: &str = "qft_call";
    pub const GROVER_ORACLE_CALL: &str = "grover_oracle_call";
    pub const MATMUL_CALL: &str = "matmul_call";
    pub const TRANSPOSE_CALL: &str = "transpose_call";
    pub const CONTRACT_CALL: &str = "contract_call";
    pub const UNCOMPUTATION_STATEMENT: &str = "uncomputation_statement";
    pub const DEFER_STATEMENT: &str = "defer_statement";
    // ... add more as needed
}

/// Helper functions for working with Naso AST nodes.
pub mod util {
    use tree_sitter::{Node, Tree};

    /// Check if a node represents a quantity-annotated type.
    pub fn is_quantity_annotated(node: &Node) -> bool {
        node.kind() == "quantity_annotation"
    }

    /// Extract the quantity from a quantity_annotation node ([0], [1], [*], [N]).
    pub fn extract_quantity(node: &Node) -> Option<String> {
        if node.kind() != "quantity_annotation" {
            return None;
        }
        node.child(1).map(|n| n.utf8_text(node.tree().text().as_bytes()).ok()).flatten()
    }

    /// Check if a function declaration has linear parameters ([1] T).
    pub fn has_linear_params(node: &Node) -> bool {
        if node.kind() != "function_declaration" {
            return false;
        }
        node.named_children(&mut node.walk()).any(|child| {
            if child.kind() == "parameter" {
                child.named_children(&mut child.walk()).any(|c| c.kind() == "quantity_annotation")
            } else {
                false
            }
        })
    }

    /// Find all quantum intrinsic calls in a tree.
    pub fn find_quantum_intrinsics(tree: &Tree) -> Vec<Node> {
        let mut results = Vec::new();
        let mut cursor = tree.walk();
        visit_for_quantum_intrinsics(&mut cursor, &mut results);
        results
    }

    fn visit_for_quantum_intrinsics(cursor: &mut tree_sitter::TreeCursor, results: &mut Vec<Node>) {
        let node = cursor.node();
        if matches!(
            node.kind(),
            "qalloc_call" | "qfree_call" | "hadamard_call" | "cnot_call" |
            "measure_call" | "bell_pair_call" | "qft_call" | "grover_oracle_call"
        ) {
            results.push(node);
        }
        if cursor.goto_first_child() {
            loop {
                visit_for_quantum_intrinsics(cursor, results);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Find all tensor operations in a tree.
    pub fn find_tensor_operations(tree: &Tree) -> Vec<Node> {
        let mut results = Vec::new();
        let mut cursor = tree.walk();
        visit_for_tensor_ops(&mut cursor, &mut results);
        results
    }

    fn visit_for_tensor_ops(cursor: &mut tree_sitter::TreeCursor, results: &mut Vec<Node>) {
        let node = cursor.node();
        if matches!(
            node.kind(),
            "matmul_call" | "add_call" | "sub_call" | "scale_call" |
            "transpose_call" | "contract_call" | "outer_product_call" | "dot_call"
        ) {
            results.push(node);
        }
        if cursor.goto_first_child() {
            loop {
                visit_for_tensor_ops(cursor, results);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Find all polyhedral loop constructs in a tree.
    pub fn find_polyhedral_loops(tree: &Tree) -> Vec<Node> {
        let mut results = Vec::new();
        let mut cursor = tree.walk();
        visit_for_polyhedral(&mut cursor, &mut results);
        results
    }

    fn visit_for_polyhedral(cursor: &mut tree_sitter::TreeCursor, results: &mut Vec<Node>) {
        let node = cursor.node();
        if matches!(node.kind(), "forall_loop" | "tile_loop" | "fuse_loop") {
            results.push(node);
        }
        if cursor.goto_first_child() {
            loop {
                visit_for_polyhedral(cursor, results);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn test_parser_creation() {
        let mut parser = Parser::new();
        assert!(parser.set_language(language()).is_ok());
    }

    #[test]
    fn test_parse_simple_function() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn main() {
    let x: [1] i32 = 42;
    qalloc();
}
"#;
        let tree = parser.parse(source, None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_quantum_intrinsics() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn quantum_circuit() {
    let q = qalloc();
    hadamard(q);
    cnot(q, q);
    measure(q);
    qfree(q);
}
"#;
        let tree = parser.parse(source, None).unwrap();
        let intrinsics = util::find_quantum_intrinsics(&tree);
        assert_eq!(intrinsics.len(), 5);
    }

    #[test]
    fn test_parse_quantity_annotations() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn linear_fn(x: [1] i32) -> [1] i32 { x }
fn proof_fn(x: [0] i32) { }
fn bounded_fn(x: [N] i32) { }
fn heap_fn(x: [*] i32) { }
"#;
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        assert_eq!(root.kind(), "source_file");
    }

    #[test]
    fn test_parse_polyhedral_loops() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn matmul_tiled(A: [N][N] f64, B: [N][N] f64) -> [N][N] f64 {
    forall(i in 0..N, j in 0..N) {
        C[i, j] = 0.0;
    }
    tile(#[schedule(tile_size=32)] (i, j)) {
        forall(k in 0..N) {
            C[i, j] += A[i, k] * B[k, j];
        }
    }
}
"#;
        let tree = parser.parse(source, None).unwrap();
        let loops = util::find_polyhedral_loops(&tree);
        assert_eq!(loops.len(), 2);
    }

    #[test]
    fn test_parse_tensor_operations() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn tensor_ops() {
    let a = matmul(x, y);
    let b = transpose(a);
    let c = contract(a, b, i, j);
}
"#;
        let tree = parser.parse(source, None).unwrap();
        let tensors = util::find_tensor_operations(&tree);
        assert_eq!(tensors.len(), 3);
    }

    #[test]
    fn test_parse_inout_parameter() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn mutate(inout x: i32) {
    x = 42;
}
"#;
        let tree = parser.parse(source, None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_uncomputation() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn quantum_with_uncompute() {
    let q = qalloc();
    hadamard(q);
    uncompute q;
}
"#;
        let tree = parser.parse(source, None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_defer() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
fn with_cleanup() {
    let x = allocate();
    defer {
        free(x);
    }
    use(x);
}
"#;
        let tree = parser.parse(source, None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_module_and_imports() {
        let mut parser = Parser::new();
        parser.set_language(language()).unwrap();
        let source = r#"
module std::quantum {
    pub fn qalloc() -> [1] qubit { ... }
    pub fn hadamard(q: [1] qubit) { ... }
}

import { qalloc, hadamard } from "std::quantum";

fn main() {
    let q = qalloc();
    hadamard(q);
}
"#;
        let tree = parser.parse(source, None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }
}