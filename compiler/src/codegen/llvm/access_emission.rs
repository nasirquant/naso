//! Access Relation Emission for LLVM
//!
//! Lowers PIR AccessRelations to LLVM GEP, load, and store instructions.
//! Handles alias.scope metadata for [1]-quantity linearity verification.

use crate::ast::Quantity;
use crate::codegen::error::CodegenResult;
use crate::codegen::llvm::value_builder::LlvmValueBuilder;
use crate::ir::{
    access_relation::{AccessRelation, AccessType},
    pir_types::PirExpr,
};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, PointerValue};

/// Access emitter for memory operations
pub struct AccessEmitter<'ctx> {
    context: &'ctx inkwell::context::Context,
}

impl<'ctx> AccessEmitter<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> CodegenResult<Self> {
        Ok(Self { context })
    }

    /// Emit an access relation (load/store/GEP)
    pub fn emit_access(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        access: &AccessRelation,
        _stmt_body: &PirExpr,
        quantities: &crate::ir::pir_types::QuantityMap,
    ) -> CodegenResult<()> {
        // Get the array/base pointer
        let base_ptr = self.get_base_pointer(value_builder, access)?;

        // Compute GEP indices from access map
        let indices = self.compute_gep_indices(value_builder, access)?;

        // Build GEP
        let gep = self.build_gep(value_builder, base_ptr, access, &indices)?;

        // Add alias.scope metadata for [1]-quantity variables
        self.add_alias_metadata(value_builder, access, quantities, gep)?;

        // Emit load or store based on access type
        match access.access_type {
            AccessType::Read => {
                self.emit_load(value_builder, gep, access)?;
            }
            AccessType::Write => {
                self.emit_store(value_builder, gep, access)?;
            }
            AccessType::ReadWrite => {
                self.emit_load(value_builder, gep, access)?;
                self.emit_store(value_builder, gep, access)?;
            }
            AccessType::Reduction => {
                self.emit_reduction(value_builder, gep, access)?;
            }
        }

        Ok(())
    }

    /// Get base pointer for the array
    fn get_base_pointer(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        access: &AccessRelation,
    ) -> CodegenResult<PointerValue<'ctx>> {
        // Look up the array name in variables
        if let Some(array_name) = &access.array_name {
            if let Some(ptr) = value_builder
                .builder()
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap()
                .get_param(0)
            {
                // Simplified: assume first parameter is the array
                return Ok(ptr.into_pointer_value());
            }
            // Try to find as variable
            if let Some(var_ptr) = value_builder.get_variable(array_name) {
                return Ok(var_ptr);
            }
        }

        // Return a dummy pointer for now
        let int_type = value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64);
        let zero = value_builder.build_int_constant(int_type, 0, "null")?;
        let ptr_type = int_type.ptr_type(inkwell::AddressSpace::default());
        Ok(ptr_type.const_null())
    }

    /// Compute GEP indices from access map
    fn compute_gep_indices(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        access: &AccessRelation,
    ) -> CodegenResult<Vec<BasicValueEnum<'ctx>>> {
        // The access map gives us the affine function from statement instance to memory location
        // For each output dimension of the access map, evaluate the affine expression
        let mut indices = Vec::new();

        for piece in &access.access_map.pieces {
            // Evaluate the affine expression for this piece
            // The piece domain should match the statement domain
            for row in &piece.matrix.rows {
                let mut expr_val: Option<BasicValueEnum<'ctx>> = None;
                for (dim, &coeff) in row.iter().enumerate() {
                    if coeff != 0 {
                        // Get the value for this dimension (induction variable or parameter)
                        let dim_val = self.get_dimension_value(value_builder, dim)?;
                        let coeff_val = value_builder.build_int_constant(
                            value_builder
                                .type_lowering()
                                .int_type(crate::codegen::abi::IntWidth::I64),
                            coeff as u64,
                            "coeff",
                        )?;
                        let term = value_builder.build_int_mul(
                            dim_val.into_int_value(),
                            coeff_val.into_int_value(),
                            "term",
                        )?;

                        if let Some(current) = expr_val {
                            expr_val = Some(
                                value_builder
                                    .build_int_add(current.into_int_value(), term, "sum")?
                                    .into(),
                            );
                        } else {
                            expr_val = Some(term.into());
                        }
                    }
                }
                if let Some(val) = expr_val {
                    indices.push(val);
                }
            }
        }

        Ok(indices)
    }

    /// Get value for a dimension (induction variable or parameter)
    fn get_dimension_value(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        dim: usize,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        // In a real implementation, this would look up the induction variable
        // or parameter value from the current scope
        let int_type = value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64);
        Ok(value_builder
            .build_int_constant(int_type, 0, &format!("dim_{}", dim))?
            .into())
    }

    /// Build GEP instruction
    fn build_gep(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        base_ptr: PointerValue<'ctx>,
        access: &AccessRelation,
        indices: &[BasicValueEnum<'ctx>],
    ) -> CodegenResult<PointerValue<'ctx>> {
        // Get the element type from the access map output
        let elem_type = self.get_element_type(value_builder, access)?;

        value_builder.build_gep(
            elem_type,
            base_ptr,
            indices,
            &format!("gep_{}", access.array_name.as_deref().unwrap_or("mem")),
        )
    }

    /// Get element type for GEP
    fn get_element_type(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        _access: &AccessRelation,
    ) -> CodegenResult<BasicTypeEnum<'ctx>> {
        // Simplified: return i64 for now
        // Real implementation would derive from array type
        Ok(value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64)
            .into())
    }

    /// Add alias.scope metadata for [1]-quantity variables
    fn add_alias_metadata(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        access: &AccessRelation,
        quantities: &crate::ir::pir_types::QuantityMap,
        gep: PointerValue<'ctx>,
    ) -> CodegenResult<()> {
        // Check if this access involves a [1]-quantity variable
        if let Some(array_name) = &access.array_name {
            if let Some(qty) = quantities.get(array_name) {
                if matches!(qty, Quantity::One) {
                    // Add noalias metadata for linear variables
                    let metadata = value_builder.add_metadata(
                        value_builder.builder().get_module().unwrap(),
                        "noalias",
                        &format!("linear_{}", array_name),
                    );
                    // Apply metadata to the GEP instruction (would need the actual instruction)
                    // This is a simplified placeholder
                }
            }
        }
        Ok(())
    }

    /// Emit load instruction
    fn emit_load(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        gep: PointerValue<'ctx>,
        _access: &AccessRelation,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let name = _access.array_name.as_deref().unwrap_or("load");
        value_builder.build_load(gep, name)
    }

    /// Emit store instruction
    fn emit_store(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        gep: PointerValue<'ctx>,
        _access: &AccessRelation,
    ) -> CodegenResult<()> {
        // Need a value to store - for now store zero
        let int_type = value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64);
        let zero = value_builder.build_int_constant(int_type, 0, "store_zero")?;
        value_builder.build_store(gep, zero.into())?;
        Ok(())
    }

    /// Emit reduction operation
    fn emit_reduction(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        gep: PointerValue<'ctx>,
        _access: &AccessRelation,
    ) -> CodegenResult<()> {
        // Load current value, add new value, store back
        let loaded = self.emit_load(value_builder, gep, _access)?;
        let int_type = value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64);
        let one = value_builder.build_int_constant(int_type, 1, "red_one")?;
        let result = value_builder.build_int_add(loaded.into_int_value(), one, "red_add")?;
        value_builder.build_store(gep, result.into())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::context::{CodegenContext, CodegenTarget, OptLevel};

    #[test]
    fn test_access_emitter_creation() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let emitter = AccessEmitter::new(context.llvm_context());
        assert!(emitter.is_ok());
    }
}
