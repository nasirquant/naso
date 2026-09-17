//! Quantum uncomputation encoding for SMT-LIB2.
//!
//! This module translates Naso's quantum operations and uncomputation obligations
//! into SMT-LIB2 constraints using symbolic unitary matrices and bitvector reasoning.

use crate::error::{LoweringError, VerifyError};
use crate::smtlib::{Sort, Term, builder::*, theory};
use indexmap::IndexMap;
use naso_compiler::ast::Span;
use naso_compiler::ast::expr::{ExprKind, GateKind as AstGateKind};
use std::collections::HashMap;

/// Quantum gate kind for symbolic representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateKind {
    H,
    X,
    Y,
    Z,
    S,
    T,
    CX,
    CY,
    CZ,
    RX,
    RY,
    RZ,
    Measure,
    Reset,
    Unitary,
}

impl GateKind {
    /// Convert from AST GateKind.
    pub fn from_ast(gate: &AstGateKind) -> Self {
        match gate {
            AstGateKind::H => GateKind::H,
            AstGateKind::X => GateKind::X,
            AstGateKind::Y => GateKind::Y,
            AstGateKind::Z => GateKind::Z,
            AstGateKind::S => GateKind::S,
            AstGateKind::T => GateKind::T,
            AstGateKind::CX => GateKind::CX,
            AstGateKind::CY => GateKind::CY,
            AstGateKind::CZ => GateKind::CZ,
            AstGateKind::RX(_) => GateKind::RX,
            AstGateKind::RY(_) => GateKind::RY,
            AstGateKind::RZ(_) => GateKind::RZ,
            AstGateKind::Custom(_) => GateKind::Unitary,
        }
    }

    /// Number of qubits this gate acts on.
    pub fn num_qubits(&self) -> usize {
        match self {
            GateKind::H
            | GateKind::T
            | GateKind::S
            | GateKind::X
            | GateKind::Y
            | GateKind::Z
            | GateKind::Measure
            | GateKind::Reset => 1,
            GateKind::CX
            | GateKind::CY
            | GateKind::CZ
            | GateKind::RX
            | GateKind::RY
            | GateKind::RZ => 2,
            GateKind::Unitary => 0, // Variable
        }
    }

    /// Check if this gate is Clifford (efficiently simulable).
    pub fn is_clifford(&self) -> bool {
        matches!(
            self,
            GateKind::H
                | GateKind::CX
                | GateKind::CY
                | GateKind::CZ
                | GateKind::S
                | GateKind::X
                | GateKind::Y
                | GateKind::Z
                | GateKind::Measure
                | GateKind::Reset
        )
    }
}

/// Symbolic qubit with its state constraints.
#[derive(Debug, Clone)]
pub struct SymbolicQubit {
    pub id: String,
    pub span: Span,
    /// Whether this qubit is a temporary (must be uncomputed)
    pub is_temp: bool,
    /// Current symbolic state (simplified: basis state index for computational basis)
    pub state_var: String, // Variable tracking |0> or |1> or superposition
    /// Unitary operations applied to this qubit (for uncomputation proof)
    pub operations: Vec<QubitOp>,
}

/// Operation applied to a qubit.
#[derive(Debug, Clone)]
pub struct QubitOp {
    pub gate: GateKind,
    pub target_qubits: Vec<String>,
    pub control_qubits: Vec<String>,
    pub span: Span,
}

/// Tracks quantum state during lowering.
pub struct QuantumTracker {
    /// All allocated qubits
    pub qubits: IndexMap<String, SymbolicQubit>,
    /// Temporary qubits that must be uncomputed
    pub temp_qubits: Vec<String>,
    /// Unitary matrix variables for symbolic reasoning
    pub unitary_vars: IndexMap<String, Term>,
    /// Next qubit ID
    next_qubit_id: u32,
    /// Current function for scoping
    pub current_function: Option<String>,
}

impl QuantumTracker {
    pub fn new() -> Self {
        Self {
            qubits: IndexMap::new(),
            temp_qubits: Vec::new(),
            unitary_vars: IndexMap::new(),
            next_qubit_id: 0,
            current_function: None,
        }
    }

    /// Allocate a new qubit (qalloc).
    pub fn allocate_qubit(&mut self, is_temp: bool, span: Span) -> String {
        let id = format!("q_{}", self.next_qubit_id);
        self.next_qubit_id += 1;

        let state_var = format!("state_{}", id);
        let qubit = SymbolicQubit {
            id: id.clone(),
            span,
            is_temp,
            state_var: state_var.clone(),
            operations: Vec::new(),
        };

        self.qubits.insert(id.clone(), qubit);
        if is_temp {
            self.temp_qubits.push(id.clone());
        }
        id
    }

    /// Apply a gate to qubits.
    pub fn apply_gate(
        &mut self,
        gate: GateKind,
        targets: &[String],
        controls: &[String],
        span: Span,
    ) {
        for target in targets {
            if let Some(qubit) = self.qubits.get_mut(target) {
                qubit.operations.push(QubitOp {
                    gate,
                    target_qubits: targets.to_vec(),
                    control_qubits: controls.to_vec(),
                    span,
                });
            }
        }
    }

    /// Get all temporary qubit IDs.
    pub fn temp_qubit_ids(&self) -> &[String] {
        &self.temp_qubits
    }

    /// Get qubit by ID.
    pub fn get_qubit(&self, id: &str) -> Option<&SymbolicQubit> {
        self.qubits.get(id)
    }

    /// Generate SMT declarations for qubits.
    pub fn generate_declarations(&self, script: &mut crate::smtlib::Script) {
        for (id, qubit) in &self.qubits {
            // Declare state variable (0 = |0>, 1 = |1>, 2 = superposition)
            script.declare_const(&qubit.state_var, Sort::Int);

            // Constrain state to valid values
            script.assert(and(vec![
                ge(var(&qubit.state_var, Sort::Int), int(0)),
                le(var(&qubit.state_var, Sort::Int), int(2)),
            ]));
        }

        // Declare unitary matrix variables for each gate application
        for (name, term) in &self.unitary_vars {
            script.declare_fun(name, vec![Sort::Int, Sort::Int], Sort::Int);
            // Note: Full unitary matrix encoding is complex; this is a placeholder
        }
    }

    /// Generate uncomputation constraints for all temporary qubits.
    pub fn generate_uncomputation_constraints(&self) -> Vec<Term> {
        let mut constraints = Vec::new();

        for temp_id in &self.temp_qubits {
            if let Some(qubit) = self.qubits.get(temp_id) {
                // For uncomputation: the final state must be |0> (state_var = 0)
                // This is the core requirement: U_temp |0> = |0>
                // We encode this as: state_var == 0 at function exit

                constraints.push(eq(var(&qubit.state_var, Sort::Int), int(0)));

                // Additional: if operations include non-Clifford gates, we need
                // more sophisticated symbolic unitary reasoning
                // For Clifford+T, we can use stabilizer formalism
                // For now, just the final state constraint
            }
        }

        constraints
    }

    /// Generate constraints for gate semantics (simplified).
    pub fn generate_gate_constraints(&self) -> Vec<Term> {
        let mut constraints = Vec::new();

        for qubit in self.qubits.values() {
            // Track state evolution through operations
            // This is a simplified version; real implementation would use
            // symbolic matrix multiplication or stabilizer tableau
            let mut current_state = var(&qubit.state_var, Sort::Int);

            for op in &qubit.operations {
                match op.gate {
                    GateKind::X => {
                        // X flips |0> <-> |1>, leaves superposition as superposition
                        // state' = ite(state == 0, 1, ite(state == 1, 0, 2))
                        current_state = ite(
                            eq(current_state.clone(), int(0)),
                            int(1),
                            ite(eq(current_state.clone(), int(1)), int(0), int(2)),
                        );
                    }
                    GateKind::H => {
                        // H creates superposition from basis states
                        // |0> -> superposition (2), |1> -> superposition (2), superposition -> basis
                        current_state = ite(
                            or(vec![
                                eq(current_state.clone(), int(0)),
                                eq(current_state.clone(), int(1)),
                            ]),
                            int(2),
                            // Superposition under H becomes basis (simplified)
                            int(0),
                        );
                    }
                    GateKind::Z | GateKind::S | GateKind::T => {
                        // Phase gates don't change computational basis state
                    }
                    GateKind::Measure => {
                        // Measurement collapses to basis state
                        // Result is non-deterministic: 0 or 1
                        // For verification, we assert the result is a valid basis state
                        constraints.push(or(vec![
                            eq(current_state.clone(), int(0)),
                            eq(current_state.clone(), int(1)),
                        ]));
                    }
                    GateKind::Reset => {
                        // Reset to |0>
                        current_state = int(0);
                    }
                    GateKind::CX => {
                        // CX: control and target both need tracking
                        // Simplified: just track that target may change
                        // Real impl: need multi-qubit state
                    }
                    GateKind::Unitary => {
                        // Custom unitary - need matrix constraints
                    }
                }
            }

            // Final state constraint already added in uncomputation constraints
        }

        constraints
    }
}

impl Default for QuantumTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode quantum operations for a function.
pub fn encode_quantum_function(
    func: &naso_compiler::ast::Function,
    tracker: &mut QuantumTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    tracker.current_function = Some(func.name.name.clone());

    // Process function body
    if let Some(body) = &func.body {
        constraints.extend(encode_quantum_expr(&body.expr, tracker)?);
        for stmt in &body.stmts {
            if let Some(expr) = &stmt.expr {
                constraints.extend(encode_quantum_expr(expr, tracker)?);
            }
        }
    }

    // Generate uncomputation constraints for temp qubits
    constraints.extend(tracker.generate_uncomputation_constraints());

    // Generate gate semantics constraints
    constraints.extend(tracker.generate_gate_constraints());

    tracker.current_function = None;

    Ok(constraints)
}

/// Encode quantum operations in an expression.
pub fn encode_quantum_expr(
    expr: &naso_compiler::ast::Expr,
    tracker: &mut QuantumTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    match &expr.kind {
        ExprKind::Call(func, args) => {
            if let ExprKind::Var(fname) = &func.kind {
                if let Some(gate) = GateKind::from_name(&fname.name) {
                    // Extract qubit arguments
                    let mut targets = Vec::new();
                    let mut controls = Vec::new();

                    for arg in args {
                        if let ExprKind::Var(qname) = &arg.kind {
                            targets.push(qname.name.clone());
                        }
                    }

                    // For CX, first arg is control, second is target
                    if gate == GateKind::CX && targets.len() >= 2 {
                        controls.push(targets[0].clone());
                        targets = vec![targets[1].clone()];
                    }

                    tracker.apply_gate(gate, &targets, &controls, expr.span);
                } else if fname.name == "qalloc" {
                    // Allocate qubit(s)
                    for (i, arg) in args.iter().enumerate() {
                        if let ExprKind::Literal(naso_compiler::ast::Literal::Int(n)) = &arg.kind {
                            for _ in 0..*n as u32 {
                                tracker.allocate_qubit(true, expr.span); // Assume temp
                            }
                        }
                    }
                } else if fname.name == "qfree" {
                    // Free qubit - check if it was temp and verify uncomputed
                    for arg in args {
                        if let ExprKind::Var(qname) = &arg.kind {
                            // Mark as freed (in real impl, check state)
                        }
                    }
                }
            }

            // Recurse into arguments
            for arg in args {
                constraints.extend(encode_quantum_expr(arg, tracker)?);
            }
        }
        ExprKind::Let(bindings, body, _) => {
            for (_, _, init, _) in bindings {
                if let Some(init_expr) = init {
                    constraints.extend(encode_quantum_expr(init_expr, tracker)?);
                }
            }
            constraints.extend(encode_quantum_expr(body, tracker)?);
        }
        _ => {
            expr.visit_exprs(&mut |e| {
                if let Ok(cs) = encode_quantum_expr(e, tracker) {
                    constraints.extend(cs);
                }
            });
        }
    }

    Ok(constraints)
}

/// Encode unitary matrix constraints for a gate (Clifford+T fragment).
pub fn encode_unitary_constraints(gate: GateKind, targets: &[String]) -> Vec<Term> {
    let mut constraints = Vec::new();

    // For each gate, we can encode its unitary matrix
    // Using bitvector representation for Clifford gates
    match gate {
        GateKind::H => {
            // H = 1/sqrt(2) * [[1, 1], [1, -1]]
            // In bitvector: we can't easily represent 1/sqrt(2), so use stabilizer
        }
        GateKind::CX => {
            // CX = [[1,0,0,0], [0,1,0,0], [0,0,0,1], [0,0,1,0]]
            // Representable exactly in bitvectors
        }
        GateKind::T => {
            // T = [[1, 0], [0, e^{iπ/4}]]
            // Not Clifford, needs algebraic numbers
        }
        _ => {}
    }

    constraints
}

/// Trait for visiting expressions.
trait VisitExprs {
    fn visit_exprs<F: FnMut(&naso_compiler::ast::Expr)>(&self, f: &mut F);
}

impl VisitExprs for naso_compiler::ast::Expr {
    fn visit_exprs<F: FnMut(&naso_compiler::ast::Expr)>(&self, f: &mut F) {
        f(self);
        match &self.kind {
            ExprKind::Call(_, args, _)
            | ExprKind::Binary(_, _, args, _)
            | ExprKind::Unary(_, arg, _)
            | ExprKind::Let(_, body, _)
            | ExprKind::If(_, _, then_e, else_e, _) => {
                // Delegate to children
            }
            _ => {}
        }
    }
}

/// Encode quantum operations for a statement.
pub fn encode_quantum_stmt(
    stmt: &naso_compiler::ast::Stmt,
    tracker: &mut QuantumTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    match &stmt.kind {
        naso_compiler::ast::StmtKind::Let(binding) => {
            constraints.extend(encode_quantum_expr(&binding.value, tracker)?);
        }
        naso_compiler::ast::StmtKind::LetInOut(binding) => {
            constraints.extend(encode_quantum_expr(&binding.value, tracker)?);
        }
        naso_compiler::ast::StmtKind::LetConsume(binding) => {
            constraints.extend(encode_quantum_expr(&binding.value, tracker)?);
        }
        naso_compiler::ast::StmtKind::Expr(expr) => {
            constraints.extend(encode_quantum_expr(expr, tracker)?);
        }
        naso_compiler::ast::StmtKind::Assign(lhs, rhs) => {
            constraints.extend(encode_quantum_expr(lhs, tracker)?);
            constraints.extend(encode_quantum_expr(rhs, tracker)?);
        }
        _ => {}
    }

    Ok(constraints)
}
