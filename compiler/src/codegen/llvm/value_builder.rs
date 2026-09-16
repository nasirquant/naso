// @generated
#[cfg(feature = "llvm")]

//! LLVM Value Builder
//!
//! Typed builders for functions, globals, metadata, and common IR patterns.

use crate::codegen::error::{CodegenError, CodegenResult};
use crate::codegen::llvm::type_lowering::LlvmTypeLowering;
use inkwell::builder::Builder;
use inkwell::values::{FunctionValue, BasicValueEnum, PointerValue, GlobalValue, BasicBlock, InstructionValue};
use inkwell::types::{BasicTypeEnum, FunctionType, StructType};
use inkwell::AddressSpace;
use inkwell::module::Module;
use std::collections::HashMap;

/// Typed value builder for LLVM IR construction
pub struct LlvmValueBuilder<'ctx> {
    builder: Builder<'ctx>,
    type_lowering: LlvmTypeLowering<'ctx>,
    /// Metadata nodes
    metadata: HashMap<String, inkwell::metadata::MetadataValue<'ctx>>,
}

impl<'ctx> LlvmValueBuilder<'ctx> {
    /// Create a new value builder
    pub fn new(builder: Builder<'ctx>, type_lowering: LlvmTypeLowering<'ctx>) -> Self {
        Self {
            builder,
            type_lowering,
            metadata: HashMap::new(),
        }
    }

    /// Get the underlying builder
    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    /// Get the type lowering
    pub fn type_lowering(&self) -> &LlvmTypeLowering<'ctx> {
        &self.type_lowering
    }

    /// Build a function with the given signature and body builder
    pub fn build_function<F>(
        &mut self,
        module: &Module<'ctx>,
        name: &str,
        ret_type: Option<BasicTypeEnum<'ctx>>,
        param_types: &[BasicTypeEnum<'ctx>],
        param_names: &[&str],
        body_builder: F,
    ) -> CodegenResult<FunctionValue<'ctx>>
    where
        F: FnOnce(&mut LlvmValueBuilder<'ctx>, &[BasicValueEnum<'ctx>]) -> CodegenResult<()>,
    {
        let fn_type = self.type_lowering.fn_type(ret_type, param_types, false);
        let function = module.add_function(name, fn_type, None);

        // Set parameter names
        for (i, param_name) in param_names.iter().enumerate() {
            if let Some(param) = function.get_nth_param(i as u32) {
                param.set_name(param_name);
            }
        }

        // Create entry block
        let entry = self.type_lowering.context().append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Collect parameter values
        let params: Vec<BasicValueEnum<'ctx>> = (0..param_types.len())
            .map(|i| function.get_nth_param(i as u32).unwrap())
            .collect();

        // Build function body
        body_builder(self, &params)?;

        // Verify function
        function.verify(true).map_err(|e| CodegenError::VerificationError(e.to_string()))?;

        Ok(function)
    }

    /// Build a function that returns void
    pub fn build_void_function<F>(
        &mut self,
        module: &Module<'ctx>,
        name: &str,
        param_types: &[BasicTypeEnum<'ctx>],
        param_names: &[&str],
        body_builder: F,
    ) -> CodegenResult<FunctionValue<'ctx>>
    where
        F: FnOnce(&mut LlvmValueBuilder<'ctx>, &[BasicValueEnum<'ctx>]) -> CodegenResult<()>,
    {
        self.build_function(module, name, None, param_types, param_names, body_builder)
    }

    /// Create an alloca instruction in the entry block
    pub fn build_alloca(&mut self, ty: BasicTypeEnum<'ctx>, name: &str) -> CodegenResult<PointerValue<'ctx>> {
        let current_fn = self.builder.get_insert_block().unwrap().get_parent().unwrap();
        let entry = current_fn.get_first_basic_block().unwrap();
        let first_inst = entry.get_first_instruction();
        
        // Save current position
        let saved_block = self.builder.get_insert_block();
        let saved_inst = self.builder.get_insert_point();
        
        // Move to entry block
        if let Some(inst) = first_inst {
            self.builder.position_before(&inst);
        } else {
            self.builder.position_at_end(entry);
        }
        
        let alloca = self.builder.build_alloca(ty, name).map_err(|e| CodegenError::InstructionError(e.to_string()))?;
        
        // Restore position
        if let (Some(block), Some(inst)) = (saved_block, saved_inst) {
            self.builder.position_before(&inst);
        } else if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }
        
        Ok(alloca)
    }

    /// Build a load instruction
    pub fn build_load(&mut self, ptr: PointerValue<'ctx>, name: &str) -> CodegenResult<BasicValueEnum<'ctx>> {
        self.builder.build_load(ptr, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build a store instruction
    pub fn build_store(&mut self, ptr: PointerValue<'ctx>, val: BasicValueEnum<'ctx>) -> CodegenResult<InstructionValue<'ctx>> {
        self.builder.build_store(ptr, val).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build a global variable
    pub fn build_global(
        &mut self,
        module: &Module<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
        init: Option<BasicValueEnum<'ctx>>,
        is_constant: bool,
    ) -> CodegenResult<GlobalValue<'ctx>> {
        let global = module.add_global(ty, None, name);
        if let Some(init_val) = init {
            global.set_initializer(&init_val);
        }
        global.set_constant(is_constant);
        Ok(global)
    }

    /// Build a global string pointer
    pub fn build_global_string(&mut self, module: &Module<'ctx>, name: &str, value: &str) -> CodegenResult<GlobalValue<'ctx>> {
        let str_val = self.builder.build_global_string_ptr(value, name).map_err(|e| CodegenError::InstructionError(e.to_string()))?;
        // The global string is already added to module by build_global_string_ptr
        // We need to find it
        let global = module.get_global(name).ok_or_else(|| CodegenError::InstructionError("Global string not found".to_string()))?;
        Ok(global)
    }

    /// Build a struct type with named fields
    pub fn build_struct_type(&mut self, name: &str, fields: &[BasicTypeEnum<'ctx>], is_packed: bool) -> StructType<'ctx> {
        self.type_lowering.get_or_create_struct(name, fields, is_packed)
    }

    /// Build a struct value (aggregate constant or runtime)
    pub fn build_struct_value(&mut self, struct_ty: StructType<'ctx>, fields: &[BasicValueEnum<'ctx>]) -> CodegenResult<BasicValueEnum<'ctx>> {
        // For runtime values, we need to allocate and store each field
        let alloca = self.build_alloca(struct_ty.into(), "struct_tmp")?;
        for (i, field) in fields.iter().enumerate() {
            let gep = self.builder.build_struct_gep(struct_ty, alloca, i as u32, "field_gep").map_err(|e| CodegenError::InstructionError(e.to_string()))?;
            self.build_store(gep, *field)?;
        }
        self.build_load(alloca, "struct_val")
    }

    /// Extract a field from a struct value
    pub fn build_extract_value(&mut self, struct_val: BasicValueEnum<'ctx>, index: u32, name: &str) -> CodegenResult<BasicValueEnum<'ctx>> {
        self.builder.build_extract_value(struct_val.into_struct_value(), index, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Insert a value into a struct
    pub fn build_insert_value(&mut self, struct_val: BasicValueEnum<'ctx>, value: BasicValueEnum<'ctx>, index: u32, name: &str) -> CodegenResult<BasicValueEnum<'ctx>> {
        self.builder.build_insert_value(struct_val.into_struct_value(), value, index, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build a GEP (getelementptr) for arrays/pointers
    pub fn build_gep(&mut self, ty: BasicTypeEnum<'ctx>, ptr: PointerValue<'ctx>, indices: &[inkwell::values::BasicValueEnum<'ctx>], name: &str) -> CodegenResult<PointerValue<'ctx>> {
        self.builder.build_gep(ty, ptr, indices, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build a struct GEP
    pub fn build_struct_gep(&mut self, struct_ty: StructType<'ctx>, ptr: PointerValue<'ctx>, index: u32, name: &str) -> CodegenResult<PointerValue<'ctx>> {
        self.builder.build_struct_gep(struct_ty, ptr, index, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build a function call
    pub fn build_call(&mut self, func: FunctionValue<'ctx>, args: &[BasicValueEnum<'ctx>], name: &str) -> CodegenResult<BasicValueEnum<'ctx>> {
        let call_site = self.builder.build_call(func, args, name).map_err(|e| CodegenError::InstructionError(e.to_string()))?;
        call_site.try_as_basic_value().left().ok_or_else(|| CodegenError::InstructionError("Call returned void".to_string()))
    }

    /// Build an indirect function call
    pub fn build_indirect_call(&mut self, fn_ty: FunctionType<'ctx>, func_ptr: PointerValue<'ctx>, args: &[BasicValueEnum<'ctx>], name: &str) -> CodegenResult<BasicValueEnum<'ctx>> {
        let call_site = self.builder.build_indirect_call(fn_ty, func_ptr, args, name).map_err(|e| CodegenError::InstructionError(e.to_string()))?;
        call_site.try_as_basic_value().left().ok_or_else(|| CodegenError::InstructionError("Indirect call returned void".to_string()))
    }

    /// Build a return instruction
    pub fn build_return(&mut self, val: Option<BasicValueEnum<'ctx>>) -> CodegenResult<InstructionValue<'ctx>> {
        self.builder.build_return(val).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build an unconditional branch
    pub fn build_unconditional_branch(&mut self, dest: BasicBlock<'ctx>) -> CodegenResult<InstructionValue<'ctx>> {
        self.builder.build_unconditional_branch(dest).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build a conditional branch
    pub fn build_conditional_branch(&mut self, cond: inkwell::values::IntValue<'ctx>, then_bb: BasicBlock<'ctx>, else_bb: BasicBlock<'ctx>) -> CodegenResult<InstructionValue<'ctx>> {
        self.builder.build_conditional_branch(cond, then_bb, else_bb).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build a PHI node
    pub fn build_phi(&mut self, ty: BasicTypeEnum<'ctx>, name: &str) -> CodegenResult<inkwell::values::PhiValue<'ctx>> {
        self.builder.build_phi(ty, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Add incoming values to a PHI node
    pub fn add_phi_incoming(&mut self, phi: &inkwell::values::PhiValue<'ctx>, values: &[(&BasicValueEnum<'ctx>, BasicBlock<'ctx>)]) {
        let incoming: Vec<_> = values.iter().map(|(v, bb)| (*v, *bb)).collect();
        phi.add_incoming(&incoming);
    }

    /// Build integer constant
    pub fn build_int_constant(&mut self, ty: inkwell::types::IntType<'ctx>, value: u64, name: &str) -> inkwell::values::IntValue<'ctx> {
        ty.const_int(value, false)
    }

    /// Build float constant
    pub fn build_float_constant(&mut self, ty: inkwell::types::FloatType<'ctx>, value: f64, name: &str) -> inkwell::values::FloatValue<'ctx> {
        ty.const_float(value)
    }

    /// Build a zero value for a type
    pub fn build_zero(&mut self, ty: BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
        ty.const_zero()
    }

    /// Build an undefined value for a type
    pub fn build_undef(&mut self, ty: BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
        ty.get_undef()
    }

    /// Build integer arithmetic
    pub fn build_int_add(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_int_add(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    pub fn build_int_sub(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_int_sub(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    pub fn build_int_mul(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_int_mul(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    pub fn build_int_signed_div(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_int_signed_div(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build integer comparison
    pub fn build_int_compare(&mut self, pred: inkwell::IntPredicate, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_int_compare(pred, lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build bitwise operations
    pub fn build_and(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_and(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    pub fn build_or(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_or(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    pub fn build_xor(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_xor(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Build shift operations
    pub fn build_left_shift(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_left_shift(lhs, rhs, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    pub fn build_right_shift(&mut self, lhs: inkwell::values::IntValue<'ctx>, rhs: inkwell::values::IntValue<'ctx>, is_arithmetic: bool, name: &str) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        self.builder.build_right_shift(lhs, rhs, is_arithmetic, name).map_err(|e| CodegenError::InstructionError(e.to_string()))
    }

    /// Add metadata to the module
    pub fn add_metadata(&mut self, module: &Module<'ctx>, kind: &str, value: &str) -> inkwell::metadata::MetadataValue<'ctx> {
        let md_string = self.builder.get_context().create_string_metadata(value);
        let md_node = self.builder.get_context().create_metadata_node(&[md_string.into()]);
        module.add_metadata(kind, &md_node);
        md_node
    }

    /// Get or create debug location
    pub fn create_debug_location(&self, line: u32, col: u32, scope: inkwell::debug_info::DIScope<'ctx>) -> inkwell::debug_info::DILocation<'ctx> {
        self.builder.get_context().create_debug_location(line, col, scope, None)
    }

    /// Set debug location for subsequent instructions
    pub fn set_debug_location(&mut self, loc: inkwell::debug_info::DILocation<'ctx>) {
        self.builder.set_current_debug_location(loc);
    }

    /// Clear debug location
    pub fn clear_debug_location(&mut self) {
        self.builder.clear_current_debug_location();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::context::{CodegenContext, CodegenTarget, OptLevel};

    #[test]
    fn test_value_builder_creation() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let llvm_context = context.llvm_context();
        let builder = llvm_context.create_builder();
        let type_lowering = crate::codegen::llvm::type_lowering::LlvmTypeLowering::new(llvm_context);
        let value_builder = LlvmValueBuilder::new(builder, type_lowering);
        // Just test it compiles
    }

    #[test]
    fn test_build_function() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let llvm_context = context.llvm_context();
        let module = llvm_context.create_module("test");
        let builder = llvm_context.create_builder();
        let type_lowering = crate::codegen::llvm::type_lowering::LlvmTypeLowering::new(llvm_context);
        let mut value_builder = LlvmValueBuilder::new(builder, type_lowering);

        let fn_val = value_builder.build_void_function(
            &module,
            "test_fn",
            &[],
            &[],
            |_, _| Ok(()),
        );
        assert!(fn_val.is_ok());
    }

    #[test]
    fn test_build_alloca() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let llvm_context = context.llvm_context();
        let module = llvm_context.create_module("test");
        let builder = llvm_context.create_builder();
        let type_lowering = crate::codegen::llvm::type_lowering::LlvmTypeLowering::new(llvm_context);
        let mut value_builder = LlvmValueBuilder::new(builder, type_lowering);

        let fn_val = value_builder.build_void_function(
            &module,
            "test_fn",
            &[],
            &[],
            |vb, _| {
                let alloca = vb.build_alloca(vb.type_lowering().context().i32_type().into(), "test_var")?;
                assert!(!alloca.is_null());
                Ok(())
            },
        );
        assert!(fn_val.is_ok());
    }
}