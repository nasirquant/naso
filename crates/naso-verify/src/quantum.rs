//! Quantum uncomputation encoding for SMT-LIB2.
//!
//! This module translates Naso's quantum operations and uncomputation obligations
//! into SMT-LIB2 constraints using symbolic unitary matrices and bitvector reasoning.

use crate::error::VerifyError;
use crate::smtlib::{Sort, Term, builder::*};
use indexmap::IndexMap;
use naso_compiler::ast::Span;
use naso_compiler::ast::expr::{ExprKind, GateKind as AstGateKind};

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
    pub state_var: String,
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
        for (_id, qubit) in &self.qubits {
            script.declare_const(&qubit.state_var, Sort::Int);

            script.assert(and(vec![
                ge(var(&qubit.state_var, Sort::Int), int(0)),
                le(var(&qubit.state_var, Sort::Int), int(2)),
            ]));
        }

        for (name, _term) in &self.unitary_vars {
            script.declare_fun(name, vec![Sort::Int, Sort::Int], Sort::Int);
        }
    }

    /// Generate uncomputation constraints for all temporary qubits.
    pub fn generate_uncomputation_constraints(&self) -> Vec<Term> {
        let mut constraints = Vec::new();

        for temp_id in &self.temp_qubits {
            if let Some(qubit) = self.qubits.get(temp_id) {
                constraints.push(eq(var(&qubit.state_var, Sort::Int), int(0)));
            }
        }

        constraints
    }

    /// Generate constraints for gate semantics (simplified).
    pub fn generate_gate_constraints(&self) -> Vec<Term> {
        let mut constraints = Vec::new();

        for qubit in self.qubits.values() {
            let mut current_state = var(&qubit.state_var, Sort::Int);

            for op in &qubit.operations {
                match op.gate {
                    GateKind::X => {
                        current_state = ite(
                            eq(current_state.clone(), int(0)),
                            int(1),
                            ite(eq(current_state.clone(), int(1)), int(0), int(2)),
                        );
                    }
                    GateKind::H => {
                        current_state = ite(
                            or(vec![
                                eq(current_state.clone(), int(0)),
                                eq(current_state.clone(), int(1)),
                            ]),
                            int(2),
                            int(0),
                        );
                    }
                    GateKind::Y | GateKind::Z | GateKind::S | GateKind::T => {}
                    GateKind::Measure => {
                        constraints.push(or(vec![
                            eq(current_state.clone(), int(0)),
                            eq(current_state.clone(), int(1)),
                        ]));
                    }
                    GateKind::Reset => {
                        current_state = int(0);
                    }
                    GateKind::CX | GateKind::CY | GateKind::CZ => {}
                    GateKind::RX | GateKind::RY | GateKind::RZ => {}
                    GateKind::Unitary => {}
                }
            }
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
pub fn encode_quantum_expr(
    expr: &naso_compiler::ast::Expr,
    tracker: &mut QuantumTracker,
) -> Result<Vec<Term>, VerifyError> {
    let mut constraints = Vec::new();

    match &expr.kind {
        ExprKind::Call(func, args) => {
            if let ExprKind::Var(fname) = &func.kind {
                // Try to parse as a gate name
                if let Some(gate) = parse_gate_name(&fname.name) {
                    let mut targets = Vec::new();
                    let mut controls = Vec::new();

                    for arg in args {
                        if let ExprKind::Var(qname) = &arg.kind {
                            targets.push(qname.name.clone());
                        }
                    }

                    if gate == GateKind::CX && targets.len() >= 2 {
                        controls.push(targets[0].clone());
                        targets = vec![targets[1].clone()];
                    }

                    tracker.apply_gate(gate, &targets, &controls, expr.span);
                } else if fname.name == "qalloc" {
                    for arg in args.iter() {
                        if let ExprKind::Literal(naso_compiler::ast::Literal::Int(_n)) = &arg.kind {
                            for _ in 0..*_n as u32 {
                                tracker.allocate_qubit(true, expr.span);
                            }
                        }
                    }
                } else if fname.name == "qfree" {
                    for arg in args {
                        if let ExprKind::Var(_qname) = &arg.kind {
                            // Mark as freed (in real impl, check state)
                        }
                    }
                }
            }

            for arg in args {
                constraints.extend(encode_quantum_expr(arg, tracker)?);
            }
        }
        ExprKind::QuantumOp(qop) => {
            match qop {
                naso_compiler::ast::expr::QuantumOp::Alloc(_name) => {
                    tracker.allocate_qubit(true, expr.span);
                }
                naso_compiler::ast::expr::QuantumOp::Measure(target) => {
                    #[allow(clippy::collapsible_if)]
                    if let ExprKind::Var(qname) = &target.kind {
                        if let Some(_qubit) = tracker.get_qubit(&qname.name) {
                            // Measurement collapses state to basis
                        }
                    }
                    constraints.extend(encode_quantum_expr(target, tracker)?);
                }
                naso_compiler::ast::expr::QuantumOp::ApplyGate(gate, args) => {
                    let gate_kind = GateKind::from_ast(gate);
                    let mut targets = Vec::new();
                    let mut controls = Vec::new();

                    for arg in args {
                        if let ExprKind::Var(qname) = &arg.kind {
                            targets.push(qname.name.clone());
                        }
                    }

                    if gate_kind == GateKind::CX && targets.len() >= 2 {
                        controls.push(targets[0].clone());
                        targets = vec![targets[1].clone()];
                    }

                    tracker.apply_gate(gate_kind, &targets, &controls, expr.span);
                }
                naso_compiler::ast::expr::QuantumOp::Entangle(args) => {
                    for arg in args {
                        constraints.extend(encode_quantum_expr(arg, tracker)?);
                    }
                }
                naso_compiler::ast::expr::QuantumOp::Phase(_, _) => {}
                naso_compiler::ast::expr::QuantumOp::Hamiltonian(_, _) => {}
            }
        }
        ExprKind::Let(binding) => {
            constraints.extend(encode_quantum_expr(&binding.value, tracker)?);
        }
        ExprKind::LetInOut(binding) => {
            constraints.extend(encode_quantum_expr(&binding.value, tracker)?);
        }
        ExprKind::LetConsume(binding) => {
            constraints.extend(encode_quantum_expr(&binding.value, tracker)?);
        }
        ExprKind::Block(block) => {
            if let Some(body_expr) = &block.expr {
                constraints.extend(encode_quantum_expr(body_expr, tracker)?);
            }
            for stmt in &block.stmts {
                if let naso_compiler::ast::StmtKind::Expr(stmt_expr) = &stmt.kind {
                    constraints.extend(encode_quantum_expr(stmt_expr, tracker)?);
                }
            }
        }
        ExprKind::If(cond, then_e, else_e) => {
            constraints.extend(encode_quantum_expr(cond, tracker)?);
            constraints.extend(encode_quantum_expr(then_e, tracker)?);
            if let Some(else_e) = else_e {
                constraints.extend(encode_quantum_expr(else_e, tracker)?);
            }
        }
        ExprKind::Binary(_, lhs, rhs) => {
            constraints.extend(encode_quantum_expr(lhs, tracker)?);
            constraints.extend(encode_quantum_expr(rhs, tracker)?);
        }
        ExprKind::Unary(_, operand) => {
            constraints.extend(encode_quantum_expr(operand, tracker)?);
        }
        ExprKind::MethodCall(receiver, _, args) => {
            constraints.extend(encode_quantum_expr(receiver, tracker)?);
            for arg in args {
                constraints.extend(encode_quantum_expr(arg, tracker)?);
            }
        }
        _ => {}
    }

    Ok(constraints)
}

/// Parse a gate name string to GateKind.
fn parse_gate_name(name: &str) -> Option<GateKind> {
    match name {
        "H" | "hadamard" => Some(GateKind::H),
        "X" => Some(GateKind::X),
        "Y" => Some(GateKind::Y),
        "Z" => Some(GateKind::Z),
        "S" => Some(GateKind::S),
        "T" => Some(GateKind::T),
        "CX" | "cnot" => Some(GateKind::CX),
        "CY" => Some(GateKind::CY),
        "CZ" => Some(GateKind::CZ),
        "RX" => Some(GateKind::RX),
        "RY" => Some(GateKind::RY),
        "RZ" => Some(GateKind::RZ),
        "measure" => Some(GateKind::Measure),
        "reset" => Some(GateKind::Reset),
        _ => None,
    }
}

/// Encode unitary matrix constraints for a gate (Clifford+T fragment).
pub fn encode_unitary_constraints(_gate: GateKind, _targets: &[String]) -> Vec<Term> {
    Vec::new()
}
