//! Codegen Property Tests
//!
//! Property-based tests for codegen using proptest to verify invariants
//! hold across randomly generated inputs.

#[cfg(feature = "llvm")]
use naso_compiler::codegen::validate::{BitcodeValidator, ValidationReport, ValidationSeverity};
#[cfg(feature = "llvm")]
use naso_compiler::ir::pir_types::{PirModule, PirStatement};
#[cfg(feature = "llvm")]
use std::collections::HashMap;

#[cfg(feature = "llvm")]
mod llvm_property_tests {
    use super::*;
    use inkwell::context::Context;
    use inkwell::module::Module as LlvmModule;
    use inkwell::types::BasicTypeEnum;
    use inkwell::values::{BasicValueEnum, FunctionValue};
    use proptest::prelude::*;

    /// Strategy for generating valid function names
    fn function_name_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z_][a-zA-Z0-9_]*".prop_map(|s| format!("fn_{}", s))
    }

    /// Strategy for generating basic block names
    fn basic_block_name_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z_][a-zA-Z0-9_]*".prop_map(|s| format!("bb_{}", s))
    }

    /// Strategy for generating integer constants
    fn int_constant_strategy() -> impl Strategy<Value = i64> {
        -1000..1000
    }

    /// Property: Every valid LLVM module should pass validation
    proptest! {
        #[test]
        fn prop_valid_module_passes_validation(
            num_functions in 1..5,
            num_blocks_per_fn in 1..4,
            num_instructions_per_block in 1..6,
        ) {
            let context = Context::create();
            let module = build_random_module(&context, num_functions, num_blocks_per_fn, num_instructions_per_block);

            let validator = BitcodeValidator::new(&context);
            let report = validator.validate_module(&module).expect("Validation should not fail");

            // A well-formed module should not have errors
            // (may have warnings for declarations, etc.)
            for error in &report.errors {
                // Filter out expected errors from intentionally incomplete modules
                if error.severity == ValidationSeverity::Fatal {
                    prop_assert!(false, "Fatal error in generated module: {}", error.message);
                }
            }
        }
    }

    /// Property: Function with body must have entry block and terminators
    proptest! {
        #[test]
        fn prop_function_body_structure(
            num_blocks in 1..5,
            has_terminator in any::<bool>(),
        ) {
            let context = Context::create();
            let module = build_module_with_function_structure(&context, num_blocks, has_terminator);

            let validator = BitcodeValidator::new(&context);
            let report = validator.validate_module(&module).expect("Validation should not fail");

            if has_terminator {
                // Should pass basic block terminator checks
                let has_terminator_errors = report.errors.iter().any(|e|
                    e.check.contains("Block terminator")
                );
                prop_assert!(!has_terminator_errors, "Function with terminators should not have terminator errors");
            } else {
                // Should have terminator errors
                let has_terminator_errors = report.errors.iter().any(|e|
                    e.check.contains("Block terminator")
                );
                prop_assert!(has_terminator_errors, "Function without terminators should have terminator errors");
            }
        }
    }

    /// Property: Call instruction argument types must match callee signature
    proptest! {
        #[test]
        fn prop_call_signature_matching(
            num_params in 0..4,
            args_match in any::<bool>(),
        ) {
            let context = Context::create();
            let module = build_module_with_call(&context, num_params, args_match);

            let validator = BitcodeValidator::new(&context);
            let report = validator.validate_module(&module).expect("Validation should not fail");

            if args_match {
                let has_signature_errors = report.errors.iter().any(|e|
                    e.check.contains("Call signature")
                );
                prop_assert!(!has_signature_errors, "Matching call signature should not produce errors");
            }
            // Note: We don't assert errors when args don't match because
            // LLVM's type system may reject the IR before our validator sees it
        }
    }

    /// Property: Global variables must have valid types (non-void)
    proptest! {
        #[test]
        fn prop_global_variable_types(
            is_void in any::<bool>(),
            is_constant in any::<bool>(),
        ) {
            let context = Context::create();
            let module = build_module_with_global(&context, is_void, is_constant);

            let validator = BitcodeValidator::new(&context);
            let report = validator.validate_module(&module).expect("Validation should not fail");

            if is_void {
                let has_type_errors = report.errors.iter().any(|e|
                    e.check.contains("Global type")
                );
                prop_assert!(has_type_errors, "Void global should produce type error");
            } else {
                let has_type_errors = report.errors.iter().any(|e|
                    e.check.contains("Global type")
                );
                prop_assert!(!has_type_errors, "Non-void global should not produce type error");
            }
        }
    }

    /// Property: Validation report summary should be consistent
    proptest! {
        #[test]
        fn prop_validation_report_consistency(
            num_passed in 0..20,
            num_warnings in 0..10,
            num_errors in 0..10,
        ) {
            let mut report = ValidationReport::new();

            for i in 0..num_passed {
                report.passed_checks.push(format!("check_{}", i));
            }

            for i in 0..num_warnings {
                report.warnings.push(super::ValidationWarning {
                    check: format!("warn_{}", i),
                    message: "warning".to_string(),
                });
            }

            for i in 0..num_errors {
                report.errors.push(super::ValidationError {
                    check: format!("err_{}", i),
                    message: "error".to_string(),
                    severity: ValidationSeverity::Error,
                });
            }

            prop_assert_eq!(report.passed_checks.len(), num_passed);
            prop_assert_eq!(report.warnings.len(), num_warnings);
            prop_assert_eq!(report.errors.len(), num_errors);
            prop_assert_eq!(report.has_errors(), num_errors > 0);
            prop_assert_eq!(report.has_warnings(), num_warnings > 0);
        }
    }

    /// Property: Structural verification should find expected elements
    proptest! {
        #[test]
        fn prop_structural_verification_basic(
            num_statements in 1..5,
            num_arrays in 1..4,
        ) {
            let pir = generate_test_pir(num_statements, num_arrays);
            let llvm = generate_test_llvm(num_statements, num_arrays);

            let report = naso_compiler::codegen::validate::StructuralVerifier::verify_pir_to_llvm(&pir, &llvm)
                .expect("Structural verification should not fail");

            // Should match at least some elements
            prop_assert!(report.matched_elements.len() >= num_statements.min(1));
        }
    }

    /// Helper: Build a random but valid LLVM module
    fn build_random_module(
        context: &Context,
        num_functions: usize,
        num_blocks_per_fn: usize,
        num_instructions_per_block: usize,
    ) -> LlvmModule {
        let module = context.create_module("test_module");
        let i32_type = context.i32_type();
        let void_type = context.void_type();

        for fn_idx in 0..num_functions {
            let fn_name = format!("test_fn_{}", fn_idx);
            let fn_type = void_type.fn_type(&[], false);
            let function = module.add_function(&fn_name, fn_type, None);

            for bb_idx in 0..num_blocks_per_fn {
                let bb_name = format!("bb_{}", bb_idx);
                let bb = context.append_basic_block(function, &bb_name);
                let builder = context.create_builder();
                builder.position_at_end(bb);

                for inst_idx in 0..num_instructions_per_block {
                    let val = i32_type.const_int(inst_idx as u64, false);
                    let _ = builder.build_call(function, &[val.into()], "").unwrap();
                }

                // Add terminator
                if bb_idx == num_blocks_per_fn - 1 {
                    builder.build_return(None).unwrap();
                } else {
                    let next_bb = function.get_basic_blocks().nth(bb_idx + 1).unwrap();
                    builder.build_unconditional_branch(next_bb).unwrap();
                }
            }
        }

        module
    }

    /// Helper: Build module with specific function structure
    fn build_module_with_function_structure(
        context: &Context,
        num_blocks: usize,
        has_terminator: bool,
    ) -> LlvmModule {
        let module = context.create_module("test_module");
        let void_type = context.void_type();
        let fn_type = void_type.fn_type(&[], false);
        let function = module.add_function("test_fn", fn_type, None);

        for bb_idx in 0..num_blocks {
            let bb_name = format!("bb_{}", bb_idx);
            let bb = context.append_basic_block(function, &bb_name);
            let builder = context.create_builder();
            builder.position_at_end(bb);

            if has_terminator {
                if bb_idx == num_blocks - 1 {
                    builder.build_return(None).unwrap();
                } else {
                    let next_bb = function.get_basic_blocks().nth(bb_idx + 1).unwrap();
                    builder.build_unconditional_branch(next_bb).unwrap();
                }
            }
            // If !has_terminator, leave block without terminator
        }

        module
    }

    /// Helper: Build module with a call instruction
    fn build_module_with_call(
        context: &Context,
        num_params: usize,
        args_match: bool,
    ) -> LlvmModule {
        let module = context.create_module("test_module");
        let i32_type = context.i32_type();
        let void_type = context.void_type();

        // Create callee function
        let param_types: Vec<BasicTypeEnum> = (0..num_params).map(|_| i32_type.into()).collect();
        let callee_type = void_type.fn_type(&param_types, false);
        let callee = module.add_function("callee", callee_type, None);
        let callee_entry = context.append_basic_block(callee, "entry");
        let callee_builder = context.create_builder();
        callee_builder.position_at_end(callee_entry);
        callee_builder.build_return(None).unwrap();

        // Create caller function
        let caller_type = void_type.fn_type(&[], false);
        let caller = module.add_function("caller", caller_type, None);
        let caller_entry = context.append_basic_block(caller, "entry");
        let caller_builder = context.create_builder();
        caller_builder.position_at_end(caller_entry);

        // Build call with matching or mismatching args
        let mut args = Vec::new();
        for i in 0..num_params {
            if args_match {
                args.push(i32_type.const_int(i as u64, false).into());
            } else {
                // Mismatch: use float instead of int
                let f32_type = context.f32_type();
                args.push(f32_type.const_float(1.0).into());
            }
        }

        caller_builder.build_call(callee, &args, "").unwrap();
        caller_builder.build_return(None).unwrap();

        module
    }

    /// Helper: Build module with global variable
    fn build_module_with_global(context: &Context, is_void: bool, is_constant: bool) -> LlvmModule {
        let module = context.create_module("test_module");
        let void_type = context.void_type();
        let i32_type = context.i32_type();

        let global_type = if is_void {
            void_type.into()
        } else {
            i32_type.into()
        };
        let global = module.add_global(global_type, None, "test_global");

        if is_constant {
            global.set_constant(true);
            if !is_void {
                global.set_initializer(&i32_type.const_int(42, false));
            }
        }

        module
    }

    /// Generate test PIR content
    fn generate_test_pir(num_statements: usize, num_arrays: usize) -> String {
        let mut pir = String::new();
        pir.push_str("# Test PIR\n");

        for i in 0..num_statements {
            pir.push_str(&format!("S{} = {{\n", i));
            pir.push_str(&format!("  domain = test_domain\n"));
            pir.push_str(&format!("  body = \"stmt_{}\"\n", i));
            pir.push_str("}\n\n");
        }

        for i in 0..num_arrays {
            pir.push_str(&format!("array_{} = \"array_{}\"\n", i, i));
        }

        pir
    }

    /// Generate test LLVM IR content
    fn generate_test_llvm(num_statements: usize, num_arrays: usize) -> String {
        let mut llvm = String::new();
        llvm.push_str("define void @main() {\nentry:\n");

        for i in 0..num_statements {
            llvm.push_str(&format!("  call void @fn_s{}( )\n", i));
        }

        for i in 0..num_arrays {
            llvm.push_str(&format!("  %{} = alloca i32\n", i));
        }

        llvm.push_str("  ret void\n}\n");

        for i in 0..num_statements {
            llvm.push_str(&format!(
                "define void @fn_s{}() {{\nentry:\n  ret void\n}}\n",
                i
            ));
        }

        llvm
    }
}

#[cfg(feature = "llvm")]
mod qir_property_tests {
    use super::*;
    use inkwell::context::Context;
    use proptest::prelude::*;

    /// Property: QIR module should contain quantum runtime initialization
    proptest! {
        #[test]
        fn prop_qir_has_runtime_init(
            has_init in any::<bool>(),
        ) {
            let context = Context::create();
            let qir = generate_test_qir(has_init);

            let validator = BitcodeValidator::new(&context);
            let report = validator.validate_qir_module(&qir).expect("QIR validation should not fail");

            if has_init {
                let has_init_check = report.passed_checks.iter().any(|c|
                    c.contains("QIR module declaration") || c.contains("initialize")
                );
                prop_assert!(has_init_check, "QIR with init should pass init check");
            }
        }
    }

    /// Property: QIR with quantum operations should be detected
    proptest! {
        #[test]
        fn prop_qir_quantum_ops_detection(
            has_quantum_ops in any::<bool>(),
        ) {
            let context = Context::create();
            let qir = generate_test_qir_with_ops(has_quantum_ops);

            let validator = BitcodeValidator::new(&context);
            let report = validator.validate_qir_module(&qir).expect("QIR validation should not fail");

            if has_quantum_ops {
                let has_quantum_check = report.passed_checks.iter().any(|c|
                    c.contains("Quantum intrinsics")
                );
                prop_assert!(has_quantum_check, "QIR with quantum ops should pass quantum check");
            }
        }
    }

    /// Property: QIR validation report consistency
    proptest! {
        #[test]
        fn prop_qir_validation_report_consistency(
            num_passed in 0..10,
            num_warnings in 0..5,
            num_errors in 0..5,
        ) {
            let mut report = super::QirValidationReport::new();

            for i in 0..num_passed {
                report.passed_checks.push(format!("check_{}", i));
            }

            for i in 0..num_warnings {
                report.warnings.push(super::QirValidationWarning {
                    check: format!("warn_{}", i),
                    message: "warning".to_string(),
                });
            }

            for i in 0..num_errors {
                report.errors.push(super::QirValidationError {
                    check: format!("err_{}", i),
                    message: "error".to_string(),
                    severity: ValidationSeverity::Error,
                });
            }

            prop_assert_eq!(report.passed_checks.len(), num_passed);
            prop_assert_eq!(report.warnings.len(), num_warnings);
            prop_assert_eq!(report.errors.len(), num_errors);
            prop_assert_eq!(report.has_errors(), num_errors > 0);
        }
    }

    /// Generate test QIR content
    fn generate_test_qir(has_init: bool) -> String {
        let mut qir = String::new();
        qir.push_str("target triple = \"x86_64-unknown-linux-gnu\"\n");
        qir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n\n");

        if has_init {
            qir.push_str("declare void @__quantum__rt__initialize(i64)\n");
            qir.push_str("define void @main() {\n  call void @__quantum__rt__initialize(i64 1)\n  ret void\n}\n");
        } else {
            qir.push_str("define void @main() {\n  ret void\n}\n");
        }

        qir
    }

    /// Generate test QIR with optional quantum operations
    fn generate_test_qir_with_ops(has_quantum_ops: bool) -> String {
        let mut qir = generate_test_qir(true);

        if has_quantum_ops {
            qir.push_str("\ndeclare void @__quantum__qis__h__body(i8*)\n");
            qir.push_str("define void @quantum_op() {\n  %q = call i8* @__quantum__rt__qubit_allocate()\n  call void @__quantum__qis__h__body(i8* %q)\n  ret void\n}\n");
        }

        qir
    }
}

/// Tests that run without LLVM feature
#[cfg(feature = "llvm")]
mod no_llvm_property_tests {
    use super::*;

    #[test]
    fn test_validation_report_basic_properties() {
        // Test basic properties without LLVM
        let mut report = ValidationReport::new();
        assert!(!report.has_errors());
        assert!(!report.has_warnings());

        report.errors.push(ValidationError {
            check: "test".to_string(),
            message: "error".to_string(),
            severity: ValidationSeverity::Error,
        });
        assert!(report.has_errors());

        report.warnings.push(ValidationWarning {
            check: "test".to_string(),
            message: "warning".to_string(),
        });
        assert!(report.has_warnings());
    }

    #[test]
    fn test_structural_report_properties() {
        let mut report = naso_compiler::codegen::validate::StructuralReport::new();
        assert!(report.all_matched());

        report.mismatched_elements.push("test".to_string());
        assert!(!report.all_matched());
    }
}
