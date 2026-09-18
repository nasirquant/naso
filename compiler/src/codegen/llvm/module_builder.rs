// LLVM Module Builder
//
// High-level wrapper around inkwell::module::Module for building LLVM IR.

use crate::ast::{Quantity, Span, Type, TypeKind};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::{CodegenError, CodegenResult};
use crate::codegen::llvm::type_lowering::LlvmTypeLowering;
use crate::codegen::llvm::value_builder::LlvmValueBuilder;
use crate::ir::pir_types::{BinaryOp, PirExpr, PirModule, PirStatement, UnaryOp};
use inkwell::AddressSpace;
use inkwell::builder::Builder as LlvmBuilder;
use inkwell::module::Module as LlvmModule;
use inkwell::types::{BasicTypeEnum, FunctionType};
use inkwell::values::{BasicBlock, BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;

/// LLVM Module Builder for constructing LLVM IR from PIR
pub struct LLVMModuleBuilder<'ctx> {
    context: &'ctx CodegenContext,
    module: LlvmModule<'ctx>,
    builder: LlvmBuilder<'ctx>,
    type_lowering: LlvmTypeLowering<'ctx>,
    value_builder: LlvmValueBuilder<'ctx>,
    /// Current function being built
    current_function: Option<FunctionValue<'ctx>>,
    /// Current basic block
    current_block: Option<BasicBlock<'ctx>>,
    /// Variable allocations (name -> pointer)
    variables: HashMap<String, PointerValue<'ctx>>,
    /// Named struct types
    struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
}

impl<'ctx> LLVMModuleBuilder<'ctx> {
    /// Create a new module builder
    pub fn new(context: &'ctx CodegenContext) -> CodegenResult<Self> {
        let llvm_context = context.llvm_context();
        let module = llvm_context.create_module("naso_module");
        let builder = llvm_context.create_builder();
        let type_lowering = LlvmTypeLowering::new(llvm_context);
        let value_builder = LlvmValueBuilder::new(builder, type_lowering.clone());

        Ok(Self {
            context,
            module,
            builder,
            type_lowering,
            value_builder,
            current_function: None,
            current_block: None,
            variables: HashMap::new(),
            struct_types: HashMap::new(),
        })
    }

    /// Get the underlying LLVM module
    pub fn module(&self) -> &LlvmModule<'ctx> {
        &self.module
    }

    /// Get the LLVM context
    pub fn llvm_context(&self) -> &inkwell::context::Context<'ctx> {
        self.context.llvm_context()
    }

    /// Get the type lowering context
    pub fn type_lowering(&mut self) -> &mut LlvmTypeLowering<'ctx> {
        &mut self.type_lowering
    }

    /// Get the value builder
    pub fn value_builder(&mut self) -> &mut LlvmValueBuilder<'ctx> {
        &mut self.value_builder
    }

    /// Set the current function
    pub fn set_current_function(&mut self, func: FunctionValue<'ctx>) {
        self.current_function = Some(func);
    }

    /// Get the current function
    pub fn current_function(&self) -> Option<FunctionValue<'ctx>> {
        self.current_function
    }

    /// Set the current basic block
    pub fn set_current_block(&mut self, block: BasicBlock<'ctx>) {
        self.current_block = Some(block);
        self.builder.position_at_end(block);
    }

    /// Get the current basic block
    pub fn current_block(&self) -> Option<BasicBlock<'ctx>> {
        self.current_block
    }

    /// Add a variable allocation
    pub fn add_variable(&mut self, name: String, ptr: PointerValue<'ctx>) {
        self.variables.insert(name, ptr);
    }

    /// Get a variable allocation
    pub fn get_variable(&self, name: &str) -> Option<PointerValue<'ctx>> {
        self.variables.get(name).copied()
    }

    /// Build the entire PIR module
    pub fn build_module(&mut self, pir_module: &PirModule) -> CodegenResult<()> {
        // Declare external functions
        for extern_fn in &pir_module.extern_functions {
            self.declare_extern_function(extern_fn)?;
        }

        // Build each statement as a function
        for stmt in &pir_module.statements {
            self.build_statement(stmt, &pir_module.quantities)?;
        }

        // Verify the module
        self.module
            .verify()
            .map_err(|e| CodegenError::VerificationError(e.to_string()))?;

        Ok(())
    }

    /// Declare an external function
    fn declare_extern_function(
        &mut self,
        extern_fn: &crate::ir::pir_types::ExternFunction,
    ) -> CodegenResult<()> {
        let param_types: CodegenResult<Vec<BasicTypeEnum<'ctx>>> = extern_fn
            .params
            .iter()
            .map(|p| {
                let ty = Type {
                    kind: TypeKind::Int, // placeholder
                    quantity: p.quantity,
                    span: Span::default(),
                };
                self.type_lowering
                    .lower_quantity_aware(&crate::codegen::abi::lower_pir_type(
                        &ty,
                        &HashMap::new(),
                    )?)
            })
            .collect();

        let param_types = param_types?;
        let ret_type = extern_fn
            .return_type
            .as_ref()
            .map(|_| self.type_lowering.void_type().into())
            .unwrap_or_else(|| self.type_lowering.void_type().into());

        let fn_type = self
            .type_lowering
            .fn_type(Some(ret_type), &param_types, false);
        self.module.add_function(&extern_fn.name, fn_type, None);
        Ok(())
    }

    /// Build a PIR statement as a function
    fn build_statement(
        &mut self,
        stmt: &PirStatement,
        quantities: &HashMap<String, Quantity>,
    ) -> CodegenResult<()> {
        let func_name = format!("stmt_{}", stmt.id.0);

        // Determine function signature based on quantities used
        let params: Vec<BasicTypeEnum<'ctx>> = Vec::new(); // Simplified for now
        let ret_type = self.type_lowering.void_type().into();
        let fn_type = self.type_lowering.fn_type(Some(ret_type), &params, false);

        let function = self.module.add_function(&func_name, fn_type, None);
        self.set_current_function(function);

        // Create entry block
        let entry = self
            .context
            .llvm_context()
            .append_basic_block(function, "entry");
        self.set_current_block(entry);

        // Build the statement body
        self.build_expr(&stmt.body, quantities)?;

        // Return void
        self.builder
            .build_return(None)
            .map_err(|e| CodegenError::InstructionError(e.to_string()))?;

        self.current_function = None;
        self.current_block = None;
        self.variables.clear();

        Ok(())
    }

    /// Build a PIR expression
    fn build_expr(
        &mut self,
        expr: &PirExpr,
        quantities: &HashMap<String, Quantity>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match expr {
            PirExpr::IntLit(val) => {
                let int_type = self
                    .type_lowering
                    .int_type(crate::codegen::abi::IntWidth::I64);
                Ok(self
                    .builder
                    .build_int_constant(int_type, *val as u64, "int_lit")
                    .unwrap()
                    .into())
            }
            PirExpr::FloatLit(val) => {
                let float_type = self
                    .type_lowering
                    .float_type(crate::codegen::abi::FloatWidth::F64);
                let parsed = val.parse::<f64>().unwrap_or(0.0);
                Ok(self
                    .builder
                    .build_float_constant(float_type, parsed, "float_lit")
                    .unwrap()
                    .into())
            }
            PirExpr::BoolLit(val) => {
                let bool_type = self
                    .type_lowering
                    .int_type(crate::codegen::abi::IntWidth::I1);
                Ok(self
                    .builder
                    .build_int_constant(bool_type, *val as u64, "bool_lit")
                    .unwrap()
                    .into())
            }
            PirExpr::Var(name) => {
                if let Some(ptr) = self.get_variable(name) {
                    let load = self
                        .builder
                        .build_load(ptr, name)
                        .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
                    Ok(load)
                } else {
                    // Return zero for undefined variables (should not happen in valid IR)
                    let int_type = self
                        .type_lowering
                        .int_type(crate::codegen::abi::IntWidth::I64);
                    Ok(self
                        .builder
                        .build_int_constant(int_type, 0, "undef")
                        .unwrap()
                        .into())
                }
            }
            PirExpr::Binary { op, left, right } => {
                let l = self.build_expr(left, quantities)?;
                let r = self.build_expr(right, quantities)?;
                self.build_binary_op(*op, l, r)
            }
            PirExpr::Unary { op, expr } => {
                let e = self.build_expr(expr, quantities)?;
                self.build_unary_op(*op, e)
            }
            PirExpr::Call { name, args } => {
                let arg_values: CodegenResult<Vec<_>> = args
                    .iter()
                    .map(|a| self.build_expr(a, quantities))
                    .collect();
                let arg_values = arg_values?;

                let func = self.module.get_function(name).ok_or_else(|| {
                    CodegenError::FunctionBuildError(format!("Function '{}' not found", name))
                })?;

                let call = self
                    .builder
                    .build_call(func, &arg_values, "call")
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
                Ok(call
                    .try_as_basic_value()
                    .left()
                    .unwrap_or_else(|| self.type_lowering.void_type().const_zero().into()))
            }
            PirExpr::Let {
                name,
                qty,
                mutability,
                value,
                body,
            } => {
                // Allocate variable
                let val = self.build_expr(value, quantities)?;
                let alloca = self
                    .builder
                    .build_alloca(val.get_type(), name)
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
                self.builder
                    .build_store(alloca, val)
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
                self.add_variable(name.clone(), alloca);

                // Build body
                let result = self.build_expr(body, quantities)?;

                // Remove variable from scope
                self.variables.remove(name);

                Ok(result)
            }
            PirExpr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.build_expr(cond, quantities)?;
                let bool_type = self
                    .type_lowering
                    .int_type(crate::codegen::abi::IntWidth::I1);
                let cond_bool = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        cond_val.into_int_value(),
                        bool_type.const_zero(),
                        "if_cond",
                    )
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;

                let func = self.current_function().unwrap();
                let then_block = self.context.llvm_context().append_basic_block(func, "then");
                let else_block = self.context.llvm_context().append_basic_block(func, "else");
                let merge_block = self
                    .context
                    .llvm_context()
                    .append_basic_block(func, "if_merge");

                self.builder
                    .build_conditional_branch(cond_bool, then_block, else_block)
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;

                // Then branch
                self.set_current_block(then_block);
                let then_val = self.build_expr(then_branch, quantities)?;
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
                let then_block_end = self.current_block().unwrap();

                // Else branch
                self.set_current_block(else_block);
                let else_val = self.build_expr(else_branch, quantities)?;
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
                let else_block_end = self.current_block().unwrap();

                // Merge block
                self.set_current_block(merge_block);
                let phi = self
                    .builder
                    .build_phi(then_val.get_type(), "if_phi")
                    .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
                phi.add_incoming(&[(&then_val, then_block_end), (&else_val, else_block_end)]);
                Ok(phi.as_basic_value())
            }
            PirExpr::Reversible { body, inverse } => {
                // For now, just build body and ignore inverse
                self.build_expr(body, quantities)
            }
            PirExpr::Index { base, indices } => {
                let base_ptr = self.build_expr(base, quantities)?;
                let index_vals: CodegenResult<Vec<_>> = indices
                    .iter()
                    .map(|i| self.build_expr(i, quantities))
                    .collect();
                let index_vals = index_vals?;

                // Simplified: just return base pointer for now
                Ok(base_ptr)
            }
            PirExpr::Field { base, field } => {
                let base_val = self.build_expr(base, quantities)?;
                // Simplified: return base for now
                Ok(base_val)
            }
        }
    }

    fn build_binary_op(
        &mut self,
        op: BinaryOp,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        use inkwell::FloatPredicate;
        use inkwell::IntPredicate;

        let left_int = left.into_int_value();
        let right_int = right.into_int_value();

        let result = match op {
            BinaryOp::Add => self.builder.build_int_add(left_int, right_int, "add"),
            BinaryOp::Sub => self.builder.build_int_sub(left_int, right_int, "sub"),
            BinaryOp::Mul => self.builder.build_int_mul(left_int, right_int, "mul"),
            BinaryOp::Div => self
                .builder
                .build_int_signed_div(left_int, right_int, "div"),
            BinaryOp::Mod => self
                .builder
                .build_int_signed_rem(left_int, right_int, "mod"),
            BinaryOp::And => self.builder.build_and(left_int, right_int, "and"),
            BinaryOp::Or => self.builder.build_or(left_int, right_int, "or"),
            BinaryOp::Xor => self.builder.build_xor(left_int, right_int, "xor"),
            BinaryOp::Eq => {
                self.builder
                    .build_int_compare(IntPredicate::EQ, left_int, right_int, "eq")
            }
            BinaryOp::Ne => {
                self.builder
                    .build_int_compare(IntPredicate::NE, left_int, right_int, "ne")
            }
            BinaryOp::Lt => {
                self.builder
                    .build_int_compare(IntPredicate::SLT, left_int, right_int, "lt")
            }
            BinaryOp::Le => {
                self.builder
                    .build_int_compare(IntPredicate::SLE, left_int, right_int, "le")
            }
            BinaryOp::Gt => {
                self.builder
                    .build_int_compare(IntPredicate::SGT, left_int, right_int, "gt")
            }
            BinaryOp::Ge => {
                self.builder
                    .build_int_compare(IntPredicate::SGE, left_int, right_int, "ge")
            }
            BinaryOp::Shl => self.builder.build_left_shift(left_int, right_int, "shl"),
            BinaryOp::Shr => self
                .builder
                .build_right_shift(left_int, right_int, true, "shr"),
        }
        .map_err(|e| CodegenError::InstructionError(e.to_string()))?;

        Ok(result.into())
    }

    fn build_unary_op(
        &mut self,
        op: UnaryOp,
        expr: BasicValueEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let int_val = expr.into_int_value();
        let result = match op {
            UnaryOp::Neg => self.builder.build_int_neg(int_val, "neg"),
            UnaryOp::Not => self.builder.build_not(int_val, "not"),
        }
        .map_err(|e| CodegenError::InstructionError(e.to_string()))?;
        Ok(result.into())
    }

    /// Convert module to LLVM IR string
    pub fn module_to_string(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Write module to .ll file
    pub fn write_ll_file(&self, path: &std::path::Path) -> CodegenResult<()> {
        self.module
            .print_to_file(path)
            .map_err(|e| CodegenError::EmissionError(e.to_string()))
    }
}

// Need to implement Clone for LlvmTypeLowering to use in value_builder
impl<'ctx> Clone for LlvmTypeLowering<'ctx> {
    fn clone(&self) -> Self {
        Self {
            context: self.context,
            struct_cache: self.struct_cache.clone(),
            qubit_type: self.qubit_type,
            result_type: self.result_type,
            quantum_address_space: self.quantum_address_space,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::context::{CodegenContext, CodegenTarget, OptLevel};

    #[test]
    fn test_module_builder_creation() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let builder = LLVMModuleBuilder::new(&context);
        assert!(builder.is_ok());
    }

    #[test]
    fn test_empty_module_emission() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let mut builder = LLVMModuleBuilder::new(&context).unwrap();

        // Create a minimal PIR module
        use crate::ast::Mutability;
        use crate::ir::pir_types::{
            AccessRelations, AffineDomain, PirModule, PirStatement, ScheduleNode, ScheduleTree,
            StmtId,
        };
        use std::collections::HashMap;

        let domain = AffineDomain::universe(0, 0);
        let schedule = ScheduleTree::new(ScheduleNode::domain(StmtId(0), domain.clone()), vec![]);
        let accesses = AccessRelations::new();
        let quantities = HashMap::new();

        let stmt = PirStatement {
            id: StmtId(0),
            domain,
            body: crate::ir::pir_types::PirExpr::IntLit(42),
            quantity: Quantity::Many,
            mutability: Mutability::Immutable,
            span: None,
        };

        let module = PirModule::new(vec![stmt], schedule, accesses, quantities, vec![]);

        let result = builder.build_module(&module);
        assert!(result.is_ok());

        let ir = builder.module_to_string();
        assert!(ir.contains("define"));
    }
}
