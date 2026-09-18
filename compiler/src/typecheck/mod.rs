//! Naso Type Checker
//!
//! Bidirectional type checker with Quantitative Type Theory (QTT) support:
//! - Quantity tracking: [0] erased, [1] linear, [N] bounded, [*] unrestricted
//! - Mutable value semantics: inout (unique mutable projection), consume (linear move)
//! - Dependent types: Pi/Sigma, Nat expressions
//! - Reversible computation with uncomputation verification
//! - Quantum linearity: Qubit/QRegister are always [1]

pub mod check;
pub mod constraints;
pub mod error;
pub mod inference;
pub mod type_env;
pub mod unify;

#[cfg(test)]
pub mod tests;

pub use error::TypeError;
pub use error::TypeResult;

use crate::ast::*;
use indexmap::IndexMap;
use std::sync::atomic::{AtomicU32, Ordering};

/// Metavariable supply for type inference
static META_VAR_SUPPLY: AtomicU32 = AtomicU32::new(0);

/// Generate a fresh metavariable
pub fn fresh_meta_var() -> MetaVar {
    MetaVar(META_VAR_SUPPLY.fetch_add(1, Ordering::Relaxed))
}

/// Generate a fresh type variable
pub fn fresh_type_var() -> TypeVar {
    TypeVar(META_VAR_SUPPLY.fetch_add(1, Ordering::Relaxed))
}

/// Bidirectional typing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Check mode: expression checks against expected type
    Check,
    /// Infer mode: expression synthesizes its type
    Infer,
}

/// Type checker state
pub struct TypeChecker {
    /// Type environment with quantity/mutability tracking
    pub env: type_env::TypeEnv,
    /// Metavariable store
    pub meta_vars: IndexMap<MetaVar, Option<Type>>,
    /// Quantity constraints to solve
    pub qty_constraints: constraints::ConstraintSet,
    /// Current typing mode
    pub mode: Mode,
    /// Errors accumulated during checking
    pub errors: Vec<error::TypeError>,
    /// Whether we're inside a reversible block (affects purity checks)
    pub in_reversible: bool,
    /// Current function signature being checked (for return type)
    pub current_fn_ret: Option<Type>,
}

impl TypeChecker {
    /// Create a new type checker with empty environment
    pub fn new() -> Self {
        Self {
            env: type_env::TypeEnv::new(),
            meta_vars: IndexMap::new(),
            qty_constraints: constraints::ConstraintSet::new(),
            mode: Mode::Infer,
            errors: Vec::new(),
            in_reversible: false,
            current_fn_ret: None,
        }
    }

    /// Create a type checker with a pre-populated environment
    pub fn with_env(env: type_env::TypeEnv) -> Self {
        Self {
            env,
            meta_vars: IndexMap::new(),
            qty_constraints: constraints::ConstraintSet::new(),
            mode: Mode::Infer,
            errors: Vec::new(),
            in_reversible: false,
            current_fn_ret: None,
        }
    }

    /// Enter check mode with expected type
    pub fn check_expr(&mut self, expr: &Expr, expected: &Type) -> Result<(), error::TypeError> {
        let prev_mode = self.mode;
        self.mode = Mode::Check;
        let result = check::check_expr(self, expr, expected);
        self.mode = prev_mode;
        result
    }

    /// Enter infer mode to synthesize type
    pub fn infer_expr(&mut self, expr: &Expr) -> Result<Type, error::TypeError> {
        let prev_mode = self.mode;
        self.mode = Mode::Infer;
        let result = inference::infer_expr(self, expr);
        self.mode = prev_mode;
        result
    }

    /// Check a statement
    pub fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), error::TypeError> {
        check::check_stmt(self, stmt)
    }

    /// Check a block
    pub fn check_block(&mut self, block: &Block) -> Result<(), error::TypeError> {
        check::check_block(self, block)
    }

    /// Check a function definition
    pub fn check_function(&mut self, func: &Function) -> Result<(), error::TypeError> {
        check::check_function(self, func)
    }

    /// Check a pattern against a scrutinee type
    pub fn check_pattern(
        &mut self,
        pattern: &Pattern,
        scrutinee_ty: &Type,
    ) -> Result<type_env::PatternBindings, error::TypeError> {
        check::check_pattern(self, pattern, scrutinee_ty)
    }

    /// Add a quantity constraint
    pub fn add_qty_constraint(&mut self, constraint: constraints::QtyConstraint) {
        self.qty_constraints.add(constraint);
    }

    /// Solve all accumulated constraints
    pub fn solve_constraints(&mut self) -> Result<(), error::TypeError> {
        constraints::solve(
            &mut self.qty_constraints,
            &mut self.env,
            &mut self.meta_vars,
        )
    }

    /// Register a metavariable with optional solution
    pub fn register_meta(&mut self, mv: MetaVar, solution: Option<Type>) {
        self.meta_vars.insert(mv, solution);
    }

    /// Lookup a metavariable solution
    pub fn lookup_meta(&self, mv: &MetaVar) -> Option<&Option<Type>> {
        self.meta_vars.get(mv)
    }

    /// Report an error
    pub fn error(&mut self, err: error::TypeError) {
        self.errors.push(err);
    }

    /// Check if any errors have been reported
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Take all errors
    pub fn take_errors(&mut self) -> Vec<error::TypeError> {
        std::mem::take(&mut self.errors)
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of type checking a program
pub struct CheckResult {
    /// The checked program (with types filled in)
    pub program: Program,
    /// Any errors encountered
    pub errors: Vec<error::TypeError>,
}

/// Type check a full program
pub fn check_program(program: &mut Program) -> CheckResult {
    let mut checker = TypeChecker::new();

    // First pass: register all top-level types and functions
    for item in &program.items {
        match item {
            Item::Function(f) => {
                checker.env.insert_function(f.clone());
            }
            Item::TypeDef(t) => {
                checker.env.insert_type_def(t.clone());
            }
            Item::Const(c) => {
                checker.env.insert_const(c.clone());
            }
            _ => {}
        }
    }

    // Second pass: check function bodies
    for item in &mut program.items {
        if let Item::Function(f) = item {
            if let Err(e) = checker.check_function(f) {
                checker.error(e);
            }
        }
    }

    // Solve constraints
    if let Err(e) = checker.solve_constraints() {
        checker.error(e);
    }

    CheckResult {
        program: program.clone(),
        errors: checker.take_errors(),
    }
}
