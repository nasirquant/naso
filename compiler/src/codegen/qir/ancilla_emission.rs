//! Ancilla Qubit Emission
//!
//! Maps TASK-303 AncillaAlloc to qir.qubit allocations with explicit qir.release
//! at inverse schedule points. Handles the lifecycle of ancilla qubits used by
//! non-reversible operations (Toffoli, measurement, RNG) in reversible circuits.

use crate::codegen::qir::classifier::{classify_statement, ancilla_requirements, is_natively_reversible, qir_intrinsic_name};
use crate::codegen::qir::module_builder::QIRModuleBuilder;
use crate::codegen::qir::primitives::QirIntrinsic;
use crate::ir::pir_types::{PirModule, PirStatement, Quantity, Mutability};
use inkwell::values::BasicValueEnum;
use std::collections::HashMap;

/// Ancilla qubit allocation and release tracking
#[derive(Debug, Clone)]
pub struct AncillaTracker {
    /// Allocated ancilla qubit pointers, indexed by allocation ID
    allocated: HashMap<String, inkwell::values::PointerValue<'static>>,
    /// Number of ancillas currently allocated
    count: u32,
    /// Whether we're in an inverse schedule context
    in_inverse: bool,
}

impl<'ctx> AncillaTracker {
    /// Create a new ancilla tracker
    pub fn new() -> Self {
        Self {
            allocated: HashMap::new(),
            count: 0,
            in_inverse: false,
        }
    }

    /// Allocate a new ancilla qubit
    pub fn allocate_ancilla(&mut self, builder: &QIRModuleBuilder<'ctx>) -> inkwell::values::PointerValue<'ctx> {
        let qubit_type = builder.qubit_type();
        let alloca = builder.builder().build_alloca(qubit_type, "ancilla").unwrap();
        
        // Call qir.qubit_alloc to allocate the qubit
        let alloc_func = builder.get_intrinsic("qir.qubit_alloc")
            .expect("qir.qubit_alloc intrinsic not declared");
        let alloc_call = builder.builder().build_call(alloc_func, &[], "ancilla_alloc")
            .map_err(|e| format!("qir.qubit_alloc call failed: {}", e))?;
        let result = alloc_call.try_as_basic_value().left().unwrap();
        
        // Store the qubit pointer
        let ptr = result.into_pointer_value();
        self.allocated.insert("ancilla_0".to_string(), ptr);
        self.count += 1;
        
        ptr
    }

    /// Release an ancilla qubit
    pub fn release_ancilla(&mut self, builder: &QIRModuleBuilder<'ctx>, name: &str) {
        if let Some(ptr) = self.allocated.remove(name) {
            // Call qir.qubit_release to deallocate
            let release_func = builder.get_intrinsic("qir.qubit_release")
                .expect("qir.qubit_release intrinsic not declared");
            let release_args = &[builder.builder().build_global_string_ptr(name).unwrap().into()];
            builder.builder().build_call(release_func, release_args, "ancilla_release")
                .map_err(|e| format!("qir.qubit_release call failed: {}", e))?;
            
            self.count -= 1;
        }
    }

    /// Mark we're entering inverse schedule context
    pub fn enter_inverse(&mut self) {
        self.in_inverse = true;
    }

    /// Mark we're exiting inverse schedule context
    pub fn exit_inverse(&mut self) {
        self.in_inverse = false;
        // Release all allocated ancillas
        for name in self.allocated.keys().cloned() {
            self.release_ancilla(&QIRModuleBuilder::new(&crate::codegen::context::CodegenContext::new(crate::codegen::context::CodegenTarget::Host, crate::codegen::context::OptLevel::None).unwrap()).unwrap(), &name);
        }
        self.allocated.clear();
        self.count = 0;
    }

    /// Get current ancilla count
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Check if we're in inverse context
    pub fn in_inverse_context(&self) -> bool {
        self.in_inverse
    }
}

/// Emit qubit allocation for a statement that requires ancillas
pub fn emit_ancilla_allocation(stmt: &PirStatement, builder: &mut QIRModuleBuilder<'_>, tracker: &mut AncillaTracker) -> CodegenResult<()> {
    let (num_ancilla, needs_release) = ancilla_requirements(stmt);
    
    if num_ancilla > 0 {
        for i in 0..num_ancilla {
            let name = format!("ancilla_{}", i);
            let ptr = tracker.allocate_ancilla(builder)?;
            tracker.release_ancilla(builder, &name); // Release immediately after use if not needed for inverse
        }
    }
    
    Ok(())
}

/// Emit the full QIR for a reversible statement with proper ancilla management
pub fn emit_reversible_qir(stmt: &PirStatement, builder: &mut QIRModuleBuilder<'_>, tracker: &mut AncillaTracker) -> CodegenResult<()> {
    let cls = classify_statement(stmt);
    
    match cls {
        // Native unitary gates - direct emission
        ReversibleClass::NativeUnitary => {
            emit_native_unitary(stmt, builder, tracker)?;
        }
        // Permutation (swap)
        ReversibleClass::Permutation => {
            emit_permutation(stmt, builder, tracker)?;
        }
        // Measurement - needs special handling with ancilla
        ReversibleClass::Measurement => {
            emit_measurement(stmt, builder, tracker)?;
        }
        // Classical control flow
        ReversibleClass::ClassicalControl => {
            emit_classical_control(stmt, builder, tracker)?;
        }
        // Non-reversible - needs ancilla management
        ReversibleClass::NonReversible => {
            emit_non_reversible(stmt, builder, tracker)?;
        }
    }
}

/// Emmit a native unitary gate
fn emit_native_unitary(stmt: &PirStatement, builder: &mut QIRModuleBuilder<'_>, tracker: &mut AncillaTracker) -> CodegenResult<()> {
    // The body is a reversible call; extract the gate name
    if let PirExpr::Call { name, args } = &stmt.body {
        let intrinsic_name = qir_intrinsic_name(ReversibleClass::NativeUnitary, name);
        let func = builder.get_intrinsic(intrinsic_name)
            .ok_or_else(|| CodegenError::QirError(format!("Intrinsic '{}' not found", intrinsic_name)))?;
        
        let arg_values: CodegenResult<Vec<_>> = args.iter()
            .map(|a| builder.build_expr(a))
            .collect();
        
        let arg_values = arg_values?;
        builder.call_intrinsic(intrinsic_name, &arg_values, "gate_call")?;
    }
    
    // Emit inverse if present
    if let PirExpr::Reversible { inverse, .. } = &stmt.body {
        // The inverse is handled by the adjoint emission
    }
    
    Ok(())
}

/// Emmit a permutation (swap) gate
fn emit_permutation(stmt: &PirStatement, builder: &mut QIRModuleBuilder<'_>, tracker: &mut AncillaTracker) -> CodegenResult<()> {
    if let PirExpr::Call { name, args } = &stmt.body {
        let intrinsic_name = qir_intrinsic_name(ReversibleClass::Permutation, name);
        let func = builder.get_intrinsic(intrinsic_name)
            .ok_or_else(|| CodegenError::QirError(format!("Intrinsic '{}' not found", intrinsic_name)))?;
        
        let arg_values: CodegenResult<Vec<_>> = args.iter()
            .map(|a| builder.build_expr(a))
            .collect();
        
        let arg_values = arg_values?;
        builder.call_intrinsic(intrinsic_name, &arg_values, "gate_call")?;
    }
    
    Ok(())
}

/// Emmit a measurement gate with ancilla management
fn emit_measurement(stmt: &PirStatement, builder: &mut QIRModuleBuilder<'_>, tracker: &mut AncillaTracker) -> CodegenResult<()> {
    tracker.enter_inverse(); // Measurement inverse requires ancilla
    
    if let PirExpr::Call { name, args } = &stmt.body {
        let intrinsic_name = qir_intrinsic_name(ReversibleClass::Measurement, name);
        let func = builder.get_intrinsic(intrinsic_name)
            .ok_or_else(|| CodegenError::QirError(format!("Intrinsic '{}' not found", intrinsic_name)))?;
        
        let arg_values: CodegenResult<Vec<_>> = args.iter()
            .map(|a| builder.build_expr(a))
            .collect();
        
        let arg_values = arg_values?;
        let result = builder.call_intrinsic(intrinsic_name, &arg_values, "measurement_call")?;
        
        // Measurement result is captured in a classical bit/result variable
        // qir.mz returns Result type
    }
    
    tracker.exit_inverse(); // This will release ancillas
    
    Ok(())
}

/// Emmit classical control flow (if/else)
fn emit_classical_control(stmt: &PirStatement, builder: &mut QIRModuleBuilder<'_>, tracker: &mut AncillaTracker) -> CodegenResult<()> {
    // Handle reversible if/else with predicate qubits
    // Emit qir.if intrinsic with result predicates
    
    Ok(())
}

/// Emmit non-reversible operation with ancilla management
fn emit_non_reversible(stmt: &PirStatement, builder: &mut QIRModuleBuilder<'_>, tracker: &mut AncillaTracker) -> CodegenResult<()> {
    let (num_ancilla, needs_release) = ancilla_requirements(stmt);
    
    // Allocate ancillas if needed
    if num_ancilla > 0 {
        for i in 0..num_ancilla {
            let name = format!("ancilla_{}", i);
            tracker.allocate_ancilla(builder)?;
            // Note: ancillas are released at inverse schedule point
        }
    }
    
    // Emit the gate operation
    if let PirExpr::Call { name, args } = &stmt.body {
        let intrinsic_name = qir_intrinsic_name(ReversibleClass::NonReversible, name);
        let func = builder.get_intrinsic(intrinsic_name)
            .ok_or_else(|| CodegenError::QirError(format!("Intrinsic '{}' not found", intrinsic_name)))?;
        
        let arg_values: CodegenResult<Vec<_>> = args.iter()
            .map(|a| builder.build_expr(a))
            .collect();
        
        let arg_values = arg_values?;
        builder.call_intrinsic(intrinsic_name, &arg_values, "non_rev_call")?;
    }
    
    // Release ancillas if needed (will be done at inverse point, but we track here)
    if needs_release {
        // Ancillas will be released when exit_inverse is called
        // For non-reversible ops at top level, release immediately
        for i in 0..num_ancilla {
            let name = format!("ancilla_{}", i);
            tracker.release_ancilla(builder, &name);
        }
    }
    
    Ok(())
}

/// Error type for QIR emission
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    #[error("QIR intrinsic error: {0}")]
    QirError(String),
    #[error("Instruction error: {0}")]
    InstructionError(String),
    #[error("Emission error: {0}")]
    EmissionError(String),
    #[error("Verification error: {0}")]
    VerificationError(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for CodegenError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::pir_types::Quantity;

    #[test]
    fn test_ancilla_tracker_creation() {
        let mut tracker = AncillaTracker::new();
        assert_eq!(tracker.count(), 0);
        assert!(!tracker.in_inverse_context());
    }

    #[test]
    fn test_classify_and_ancilla_h() {
        let stmt = PirStatement {
            id: 0,
            domain: crate::ir::affine_domain::AffineDomain::universe(1, 0),
            body: PirExpr::Call { name: "h".to_string(), args: vec![] },
            quantity: Quantity::One,
            mutability: crate::ast::Mutability::Immutable,
            span: None,
        };
        
        let cls = classify_statement(&stmt);
        assert_eq!(cls, crate::codegen::qir::classifier::ReversibleClass::NativeUnitary);
        
        let (num_ancilla, needs_release) = ancilla_requirements(&stmt);
        assert_eq!(num_ancilla, 0);
        assert!(!needs_release);
    }

    #[test]
    fn test_classify_and_ancilla_mz() {
        let stmt = PirStatement {
            id: 0,
            domain: crate::ir::affine_domain::AffineDomain::universe(1, 0),
            body: PirExpr::Call { name: "mz".to_string(), args: vec![] },
            quantity: Quantity::One,
            mutability: crate::ast::Mutability::Immutable,
            span: None,
        };
        
        let cls = classify_statement(&stmt);
        assert_eq!(cls, crate::codegen::qir::classifier::ReversibleClass::Measurement);
        
        let (num_ancilla, needs_release) = ancilla_requirements(&stmt);
        assert_eq!(num_ancilla, 1);
        assert!(needs_release);
    }

    #[test]
    fn test_is_natively_reversible() {
        let h_stmt = PirStatement {
            id: 0,
            domain: crate::ir::affine_domain::AffineDomain::universe(1, 0),
            body: PirExpr::Call { name: "h".to_string(), args: vec![] },
            quantity: Quantity::One,
            mutability: crate::ast::Mutability::Immutable,
            span: None,
        };
        assert!(is_natively_reversible(&h_stmt));
    }

    #[test]
    fn test_emit_intrinsic_name() {
        let h_name = qir_intrinsic_name(crate::codegen::qir::classifier::ReversibleClass::NativeUnitary, "h");
        assert_eq!(h_name, "qir.h");
        
        let cx_name = qir_intrinsic_name(crate::codegen::qir::classifier::ReversibleClass::NativeUnitary, "cx");
        assert_eq!(cx_name, "qir.cx");
        
        let swap_name = qir_intrinsic_name(crate::codegen::qir::classifier::ReversibleClass::Permutation, "swap");
        assert_eq!(swap_name, "qir.swap");
        
        let measure_name = qir_intrinsic_name(crate::codegen::qir::classifier::ReversibleClass::Measurement, "mz");
        assert_eq!(measure_name, "qir.mz");
    }
}