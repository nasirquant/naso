// Integration tests for the prover module.
use naso_compiler::ast::Program;
use naso_compiler::parser::parse_program;
use naso_verify::{
    prove_custom_vc, prove_linearity, prove_uncomputation, run_all_provers, run_linearity_prover,
    run_uncomputation_prover,
};

#[test]
fn test_prover_module_compiles() {
    // Smoke test - all prover functions should be accessible
    let _ = prove_uncomputation;
    let _ = prove_linearity;
    let _ = prove_custom_vc;
    let _ = run_all_provers;
    let _ = run_uncomputation_prover;
    let _ = run_linearity_prover;
}

#[test]
fn test_uncomputation_bell_pair() {
    // Test uncomputation prover on a valid Bell pair circuit
    let source = r#"
    fn bell_pair() -> [1] Qubit {
        let q0 = qalloc(1);
        let q1 = qalloc(1);
        hadamard(q0);
        cnot(q0, q1);
        // Qubits are returned, should be uncomputed automatically
        (q0, q1)
    }
    "#;
    let program = parse_program(source).expect("Parse failed");
    let diagnostics = prove_uncomputation(&program).expect("Prover failed");
    // Valid Bell pair should have no uncomputation errors
    assert!(diagnostics.is_empty());
}

#[test]
fn test_uncomputation_failed() {
    // Test uncomputation prover on invalid circuit (missing uncomputation)
    let source = r#"
    fn bad_circuit() {
        let q = qalloc(1);
        hadamard(q);
        // q not uncomputed before scope exit!
    }
    "#;
    let program = parse_program(source).expect("Parse failed");
    let diagnostics = prove_uncomputation(&program).expect("Prover failed");
    // Should detect uncomputation failure
    assert!(!diagnostics.is_empty());
    assert!(diagnostics.iter().any(|d| d.code == "NASO-UNC-001"));
}

#[test]
fn test_linearity_valid() {
    // Test linearity prover on valid linear resource usage
    let source = r#"
    fn consume_once(x: [1] i32) -> i32 {
        linear_free(x)
    }
    "#;
    let program = parse_program(source).expect("Parse failed");
    let diagnostics = prove_linearity(&program).expect("Prover failed");
    // Valid consume-once should have no linearity errors
    assert!(diagnostics.is_empty());
}

#[test]
fn test_linearity_leak() {
    // Test linearity prover on leak (unused linear resource)
    let source = r#"
    fn leak(x: [1] i32) {
        // x never consumed!
    }
    "#;
    let program = parse_program(source).expect("Parse failed");
    let diagnostics = prove_linearity(&program).expect("Prover failed");
    // Should detect leak
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "NASO-LIN-001" || d.code == "NASO-LIN-003")
    );
}

#[test]
fn test_linearity_double_use() {
    // Test linearity prover on double-use
    let source = r#"
    fn double_use(x: [1] i32) {
        linear_free(x);
        linear_free(x); // Double free!
    }
    "#;
    let program = parse_program(source).expect("Parse failed");
    let diagnostics = prove_linearity(&program).expect("Prover failed");
    // Should detect double use
    assert!(!diagnostics.is_empty());
    assert!(diagnostics.iter().any(|d| d.code == "NASO-LIN-002"));
}
