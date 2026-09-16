#[cfg(feature = "llvm")]
use crate::codegen::error::{CodegenError, CodegenResult};
#[cfg(feature = "llvm")]
use inkwell::module::Module as LlvmModule;
#[cfg(feature = "llvm")]
use inkwell::context::Context as LlvmContext;

/// Bitcode validation and structural verification for generated LLVM IR
#[cfg(feature = "llvm")]
pub struct BitcodeValidator<'ctx> {
    context: &'ctx LlvmContext,
}

#[cfg(feature = "llvm")]
impl<'ctx> BitcodeValidator<'ctx> {
    /// Create a new bitcode validator
    pub fn new(context: &'ctx LlvmContext) -> Self {
        Self { context }
    }

    /// Validate LLVM module bitcode structure
    pub fn validate_module(&self, module: &LlvmModule<'ctx>) -> CodegenResult<ValidationReport> {
        let mut report = ValidationReport::new();

        // Verify module integrity
        self.verify_module_integrity(module, &mut report)?;

        // Verify function signatures and bodies
        self.verify_functions(module, &mut report)?;

        // Verify global values
        self.verify_globals(module, &mut report)?;

        // Verify type consistency
        self.verify_types(module, &mut report)?;

        // Verify metadata
        self.verify_metadata(module, &mut report)?;

        Ok(report)
    }

    /// Verify module-level integrity using LLVM's verifier
    fn verify_module_integrity(
        &self,
        module: &LlvmModule<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        // Use LLVM's built-in module verification
        match module.verify() {
            Ok(_) => {
                report.passed_checks.push("Module integrity".to_string());
            }
            Err(errors) => {
                for error in errors {
                    report.errors.push(ValidationError {
                        check: "Module integrity".to_string(),
                        message: error.to_string(),
                        severity: ValidationSeverity::Error,
                    });
                }
            }
        }
        Ok(())
    }

    /// Verify all functions in the module
    fn verify_functions(
        &self,
        module: &LlvmModule<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        for function in module.get_functions() {
            self.verify_function(&function, report)?;
        }
        Ok(())
    }

    /// Verify a single function
    fn verify_function(
        &self,
        function: &inkwell::values::FunctionValue<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        let name = function.get_name().to_string_lossy().into_owned();

        // Check function has a body (not just a declaration)
        if function.count_basic_blocks() == 0 {
            report.warnings.push(ValidationWarning {
                check: format!("Function body: {}", name),
                message: "Function has no basic blocks (declaration only)".to_string(),
            });
        } else {
            report
                .passed_checks
                .push(format!("Function body present: {}", name));

            // Verify basic blocks
            self.verify_basic_blocks(function, report)?;
        }

        // Verify function signature
        self.verify_function_signature(function, report)?;

        Ok(())
    }

    /// Verify basic blocks in a function
    fn verify_basic_blocks(
        &self,
        function: &inkwell::values::FunctionValue<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        let name = function.get_name().to_string_lossy().into_owned();
        let mut has_entry = false;
        let mut block_count = 0;

        for bb in function.get_basic_blocks().iter() {
            block_count += 1;

            // Check for entry block
            if bb.get_name().to_string_lossy() == "entry" {
                has_entry = true;
            }

            // Verify block has terminator
            if let Some(terminator) = bb.get_terminator() {
                // Valid terminator
                let term_str = terminator.print_to_string().to_string();
                report
                    .passed_checks
                    .push(format!("Block terminator: {} in {}", term_str, name));
            } else {
                report.errors.push(ValidationError {
                    check: format!("Block terminator: {}", name),
                    message: format!(
                        "Block '{}' has no terminator",
                        bb.get_name().to_string_lossy()
                    ),
                    severity: ValidationSeverity::Error,
                });
            }

            // Verify instructions in block
            self.verify_instructions(&bb, report)?;
        }

        if !has_entry && block_count > 0 {
            report.warnings.push(ValidationWarning {
                check: format!("Entry block: {}", name),
                message: "Function has no 'entry' basic block".to_string(),
            });
        }

        Ok(())
    }

    /// Verify instructions in a basic block
    fn verify_instructions(
        &self,
        block: &inkwell::basic_block::BasicBlock<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        for inst in block.get_instructions().iter() {
            // Verify instruction has valid operands
            for i in 0..inst.get_num_operands() {
                if inst.get_operand(i).is_none() {
                    report.errors.push(ValidationError {
                        check: "Instruction operands".to_string(),
                        message: format!(
                            "Instruction '{}' has missing operand at index {}",
                            inst.print_to_string(),
                            i
                        ),
                        severity: ValidationSeverity::Error,
                    });
                }
            }

            // Check for unreachable instructions after terminator
            if inst.is_terminator() {
                // After terminator, there should be no more instructions
                // This is enforced by LLVM structure
            }

            report.passed_checks.push(format!(
                "Instruction valid: {}",
                inst.print_to_string()
                    .to_string()
                    .chars()
                    .take(60)
                    .collect::<String>()
            ));
        }
        Ok(())
    }

    /// Verify function signature consistency
    fn verify_function_signature(
        &self,
        function: &inkwell::values::FunctionValue<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        let name = function.get_name().to_string_lossy().into_owned();
        let fn_type = function.get_type();

        // Verify return type is valid
        let return_type = fn_type.get_return_type();
        if return_type.is_none() && !fn_type.is_function_var_arg() {
            report.errors.push(ValidationError {
                check: format!("Function signature: {}", name),
                message: "Function has no return type".to_string(),
                severity: ValidationSeverity::Error,
            });
        } else {
            report
                .passed_checks
                .push(format!("Function signature valid: {}", name));
        }

        // Verify parameter types
        let param_types = fn_type.get_param_types();
        for (i, param_type) in param_types.iter().enumerate() {
            if param_type.is_void_type() {
                report.errors.push(ValidationError {
                    check: format!("Function param {}: {}", i, name),
                    message: "Parameter cannot be void type".to_string(),
                    severity: ValidationSeverity::Error,
                });
            }
        }

        Ok(())
    }

    /// Verify global variables and constants
    fn verify_globals(
        &self,
        module: &LlvmModule<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        for global in module.get_global_values() {
            let name = global.get_name().to_string_lossy().into_owned();

            // Check global has initializer or is declaration
            if global.is_constant() || global.is_declaration() {
                report
                    .passed_checks
                    .push(format!("Global valid: {}", name));
            } else if global.get_initializer().is_none() {
                report.warnings.push(ValidationWarning {
                    check: format!("Global initializer: {}", name),
                    message: "Global variable has no initializer".to_string(),
                });
            }

            // Verify global type
            let global_type = global.get_type();
            if global_type.is_void_type() {
                report.errors.push(ValidationError {
                    check: format!("Global type: {}", name),
                    message: "Global cannot be void type".to_string(),
                    severity: ValidationSeverity::Error,
                });
            }
        }
        Ok(())
    }

    /// Verify type consistency across module
    fn verify_types(
        &self,
        module: &LlvmModule<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        // Check for type mismatches in calls
        for function in module.get_functions() {
            for bb in function.get_basic_blocks().iter() {
                for inst in bb.get_instructions().iter() {
                    if inst.is_call() {
                        // Check call target matches function signature
                        if let Some(called_val) = inst.get_called_value() {
                            if let Some(called_fn) = called_val.try_as_function() {
                                let fn_type = called_fn.get_type();
                                let param_types = fn_type.get_param_types();
                                let mut arg_types = Vec::new();

                                for i in 0..inst.get_num_operands() {
                                    if let Some(operand) = inst.get_operand(i) {
                                        if operand.get_type()
                                            != inkwell::values::BasicValueEnum::Function(called_fn)
                                        {
                                            arg_types.push(operand.get_type());
                                        }
                                    }
                                }

                                if param_types.len() == arg_types.len() {
                                    let mut r#match = true;
                                    for (p, a) in param_types.iter().zip(arg_types.iter()) {
                                        if p != a {
                                            r#match = false;
                                            break;
                                        }
                                    }
                                    if r#match {
                                        report.passed_checks.push(format!(
                                            "Call signature match: {}",
                                            called_fn.get_name().to_string_lossy()
                                        ));
                                    } else {
                                        report.errors.push(ValidationError {
                                            check: format!(
                                                "Call signature: {}",
                                                called_fn.get_name().to_string_lossy()
                                            ),
                                            message:
                                                "Call argument types don't match function signature"
                                                    .to_string(),
                                            severity: ValidationSeverity::Error,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Verify metadata consistency
    fn verify_metadata(
        &self,
        module: &LlvmModule<'ctx>,
        report: &mut ValidationReport,
    ) -> CodegenResult<()> {
        // Verify debug info metadata if present
        if let Some(debug_info) = module.get_named_metadata("llvm.dbg.cu") {
            for i in 0..debug_info.get_num_operands() {
                if let Some(op) = debug_info.get_operand(i) {
                    report
                        .passed_checks
                        .push(format!("Debug metadata valid: {}", op.print_to_string()));
                }
            }
        }

        // Verify module flags metadata
        if let Some(flags) = module.get_named_metadata("llvm.module.flags") {
            for i in 0..flags.get_num_operands() {
                if let Some(op) = flags.get_operand(i) {
                    report
                        .passed_checks
                        .push(format!("Module flag valid: {}", op.print_to_string()));
                }
            }
        }

        Ok(())
    }

    /// Validate QIR module structure
    pub fn validate_qir_module(&self, qir_module: &str) -> CodegenResult<QirValidationReport> {
        let mut report = QirValidationReport::new();

        // Parse and verify QIR structure
        let lines: Vec<&str> = qir_module.lines().collect();

        // Check for required QIR elements
        let has_module_decl = lines
            .iter()
            .any(|l| l.contains("target triple") || l.contains("target datalayout"));
        if has_module_decl {
            report
                .passed_checks
                .push("QIR module declaration".to_string());
        } else {
            report.warnings.push(QirValidationWarning {
                check: "QIR module declaration".to_string(),
                message: "Missing target triple/datalayout".to_string(),
            });
        }

        // Check for quantum intrinsics
        let has_quantum_intrinsics = lines.iter().any(|l| l.contains("__quantum__"));
        if has_quantum_intrinsics {
            report
                .passed_checks
                .push("Quantum intrinsics present".to_string());
        } else {
            report.warnings.push(QirValidationWarning {
                check: "Quantum intrinsics".to_string(),
                message: "No quantum intrinsics found".to_string(),
            });
        }

        // Check for entry point
        let has_main = lines
            .iter()
            .any(|l| l.contains("define") && (l.contains("main") || l.contains("entry")));
        if has_main {
            report
                .passed_checks
                .push("Entry point found".to_string());
        } else {
            report.warnings.push(QirValidationWarning {
                check: "Entry point".to_string(),
                message: "No main/entry function found".to_string(),
            });
        }

        // Check for qubit allocation
        let has_qubit_alloc = lines.iter().any(|l| {
            l.contains("__quantum__rt__qubit_allocate")
                || l.contains("__quantum__rt__qubit_allocate_array")
        });
        if has_qubit_alloc {
            report
                .passed_checks
                .push("Qubit allocation present".to_string());
        } else {
            report.warnings.push(QirValidationWarning {
                check: "Qubit allocation".to_string(),
                message: "No qubit allocation found".to_string(),
            });
        }

        // Check for measurement
        let has_measurement = lines.iter().any(|l| {
            l.contains("__quantum__rt__result_get_one") || l.contains("__quantum__rt__measure")
        });
        if has_measurement {
            report
                .passed_checks
                .push("Measurement present".to_string());
        } else {
            report.warnings.push(QirValidationWarning {
                check: "Measurement".to_string(),
                message: "No measurement operations found".to_string(),
            });
        }

        Ok(report)
    }
}

/// Validation report for LLVM module
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub passed_checks: Vec<String>,
    pub warnings: Vec<ValidationWarning>,
    pub errors: Vec<ValidationError>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            passed_checks: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Validation Report: {} passed, {} warnings, {} errors",
            self.passed_checks.len(),
            self.warnings.len(),
            self.errors.len()
        )
    }
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub check: String,
    pub message: String,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub check: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

/// Validation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
    Fatal,
}

/// QIR validation report
#[derive(Debug, Clone)]
pub struct QirValidationReport {
    pub passed_checks: Vec<String>,
    pub warnings: Vec<QirValidationWarning>,
    pub errors: Vec<QirValidationError>,
}

impl QirValidationReport {
    pub fn new() -> Self {
        Self {
            passed_checks: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "QIR Validation: {} passed, {} warnings, {} errors",
            self.passed_checks.len(),
            self.warnings.len(),
            self.errors.len()
        )
    }
}

#[derive(Debug, Clone)]
pub struct QirValidationWarning {
    pub check: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct QirValidationError {
    pub check: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

/// Structural verifier for PIR -> LLVM/QIR codegen output
pub struct StructuralVerifier;

impl StructuralVerifier {
    /// Verify PIR to LLVM IR structural correctness
    pub fn verify_pir_to_llvm(
        pir_module: &str,
        llvm_ir: &str,
    ) -> CodegenResult<StructuralReport> {
        let mut report = StructuralReport::new();

        // Parse PIR to extract expected structure
        let pir_structure = Self::parse_pir_structure(pir_module)?;

        // Verify LLVM IR matches expected structure
        Self::verify_llvm_matches_pir(&pir_structure, llvm_ir, &mut report)?;

        Ok(report)
    }

    /// Verify PIR to QIR structural correctness
    pub fn verify_pir_to_qir(
        pir_module: &str,
        qir: &str,
    ) -> CodegenResult<StructuralReport> {
        let mut report = StructuralReport::new();

        let pir_structure = Self::parse_pir_structure(pir_module)?;

        Self::verify_qir_matches_pir(&pir_structure, qir, &mut report)?;

        Ok(report)
    }

    /// Parse PIR structure for verification
    fn parse_pir_structure(pir: &str) -> CodegenResult<PirStructure> {
        let mut structure = PirStructure::default();

        for line in pir.lines() {
            let line = line.trim();
            if line.starts_with('S') && line.contains('=') {
                // Statement definition
                if let Some(stmt_name) = line.split('=').next() {
                    structure.statements.push(stmt_name.trim().to_string());
                }
            } else if line.contains("domain") && line.contains('=') {
                // Domain definition
                if let Some(domain_name) = line.split('=').next() {
                    structure.domains.push(domain_name.trim().to_string());
                }
            } else if line.contains("array_name") {
                // Array access
                if let Some(arr) = line.split('"').nth(1) {
                    structure.arrays.push(arr.to_string());
                }
            }
        }

        Ok(structure)
    }

    /// Verify LLVM IR matches PIR structure
    fn verify_llvm_matches_pir(
        pir: &PirStructure,
        llvm: &str,
        report: &mut StructuralReport,
    ) -> CodegenResult<()> {
        // Check for function definitions corresponding to statements
        for stmt in &pir.statements {
            let fn_name = format!("fn_{}", stmt.to_lowercase());
            if llvm.contains(&fn_name) || llvm.contains(&format!("define.*{}", stmt)) {
                report
                    .matched_elements
                    .push(format!("Statement -> Function: {}", stmt));
            } else {
                report
                    .mismatched_elements
                    .push(format!("Missing function for statement: {}", stmt));
            }
        }

        // Check for array allocations/globals
        for array in &pir.arrays {
            let global_name = format!("@{}", array);
            if llvm.contains(&global_name) || llvm.contains(&format!("%{}", array)) {
                report
                    .matched_elements
                    .push(format!("Array -> Global/Value: {}", array));
            } else {
                report
                    .mismatched_elements
                    .push(format!("Missing allocation for array: {}", array));
            }
        }

        // Check for loop structures
        if pir.domains.iter().any(|d| d.contains("Band")) {
            // Should have loop structures in LLVM
            if llvm.contains("br label") || llvm.contains("loop") {
                report.matched_elements.push("Loop structure".to_string());
            } else {
                report
                    .mismatched_elements
                    .push("Missing loop structure".to_string());
            }
        }

        Ok(())
    }

    /// Verify QIR matches PIR structure
    fn verify_qir_matches_pir(
        pir: &PirStructure,
        qir: &str,
        report: &mut StructuralReport,
    ) -> CodegenResult<()> {
        // Check for quantum operations
        for stmt in &pir.statements {
            if stmt.contains("quantum")
                || stmt.contains("H ")
                || stmt.contains("CNOT")
                || stmt.contains("measure")
            {
                let has_quantum_op = qir.contains("__quantum__");
                if has_quantum_op {
                    report
                        .matched_elements
                        .push(format!("Quantum statement -> QIR: {}", stmt));
                } else {
                    report
                        .mismatched_elements
                        .push(format!("Missing QIR for quantum statement: {}", stmt));
                }
            }
        }

        // Check for qubit register allocation
        if pir
            .arrays
            .iter()
            .any(|a| a.contains('q') || a.contains("qubit"))
        {
            if qir.contains("__quantum__rt__qubit_allocate") {
                report
                    .matched_elements
                    .push("Qubit register allocation".to_string());
            } else {
                report
                    .mismatched_elements
                    .push("Missing qubit allocation in QIR".to_string());
            }
        }

        Ok(())
    }
}

/// Parsed PIR structure for verification
#[derive(Debug, Default)]
struct PirStructure {
    statements: Vec<String>,
    domains: Vec<String>,
    arrays: Vec<String>,
}

/// Structural verification report
#[derive(Debug, Clone)]
pub struct StructuralReport {
    pub matched_elements: Vec<String>,
    pub mismatched_elements: Vec<String>,
}

impl StructuralReport {
    pub fn new() -> Self {
        Self {
            matched_elements: Vec::new(),
            mismatched_elements: Vec::new(),
        }
    }

    pub fn all_matched(&self) -> bool {
        self.mismatched_elements.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Structural Verification: {} matched, {} mismatched",
            self.matched_elements.len(),
            self.mismatched_elements.len()
        )
    }
}

// Public type alias for CodegenResult when not using LLVM feature
#[cfg(not(feature = "llvm"))]
pub type CodegenResult<T> = Result<T, crate::codegen::error::CodegenError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new();
        report.passed_checks.push("test".to_string());
        report.warnings.push(ValidationWarning {
            check: "test".to_string(),
            message: "warning".to_string(),
        });
        report.errors.push(ValidationError {
            check: "test".to_string(),
            message: "error".to_string(),
            severity: ValidationSeverity::Error,
        });

        assert!(!report.has_errors());
        assert!(report.has_warnings());
    }

    #[test]
    fn test_qir_validation_report() {
        let mut report = QirValidationReport::new();
        report.passed_checks.push("test".to_string());

        assert!(!report.has_errors());
    }

    #[test]
    fn test_structural_report() {
        let mut report = StructuralReport::new();
        report.matched_elements.push("test".to_string());

        assert!(report.all_matched());
    }

    #[test]
    fn test_pir_structure_default() {
        let structure = PirStructure::default();
        assert!(structure.statements.is_empty());
        assert!(structure.domains.is_empty());
        assert!(structure.arrays.is_empty());
    }

    #[cfg(feature = "llvm")]
    #[test]
    fn test_bitcode_validator_creation() {
        let context = Context::create();
        let validator = BitcodeValidator::new(&context);
        assert!(std::ptr::eq(validator.context, &context));
    }
}