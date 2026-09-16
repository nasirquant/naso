//! ABI and Type Lowering - LLVM Implementation
//!
//! LLVM-specific lowering of quantity-aware types to inkwell types.

#[cfg(feature = "llvm")]
use crate::codegen::abi_core::{QuantityAwareType, LlvmAggregateType, LlvmPointerType, IntWidth, FloatWidth};
#[cfg(feature = "llvm")]
use crate::codegen::error::{CodegenError, CodegenResult};
#[cfg(feature = "llvm")]
use crate::codegen::llvm::type_lowering::LlvmTypeLowering;
#[cfg(feature = "llvm")]
use inkwell::types::{BasicType, BasicTypeEnum, StructType, PointerType, VoidType, IntType, FloatType, ArrayType};
#[cfg(feature = "llvm")]
use inkwell::context::Context as LlvmContext;

#[cfg(feature = "llvm")]
impl QuantityAwareType {
    /// Get the LLVM type for this quantity-aware type
    pub fn to_llvm_type<'ctx>(&self, lowering: &LlvmTypeLowering<'ctx>) -> CodegenResult<BasicTypeEnum<'ctx>> {
        match self {
            QuantityAwareType::Erased => Err(CodegenError::TypeLoweringError("Cannot lower erased type to LLVM".to_string())),
            QuantityAwareType::Linear(agg) => agg.to_llvm_type(lowering),
            QuantityAwareType::Unrestricted(ptr) => ptr.to_llvm_type(lowering),
            QuantityAwareType::Qubit => Ok(lowering.qubit_type().into()),
            QuantityAwareType::Result => Ok(lowering.result_type().into()),
        }
    }
}

#[cfg(feature = "llvm")]
impl LlvmAggregateType {
    fn to_llvm_type<'ctx>(&self, lowering: &LlvmTypeLowering<'ctx>) -> CodegenResult<BasicTypeEnum<'ctx>> {
        match self {
            LlvmAggregateType::Int(width) => Ok(lowering.int_type(*width).into()),
            LlvmAggregateType::Float(width) => Ok(lowering.float_type(*width).into()),
            LlvmAggregateType::Struct(fields) => {
                let field_types: CodegenResult<Vec<_>> = fields.iter().map(|f| f.to_llvm_type(lowering)).collect();
                let struct_type = lowering.context().struct_type(&field_types?, false);
                Ok(struct_type.into())
            }
            LlvmAggregateType::Array(elem, size) => {
                let elem_type = elem.to_llvm_type(lowering)?;
                let array_type = elem_type.array_type(*size as u32);
                Ok(array_type.into())
            }
            LlvmAggregateType::Tuple(elems) => {
                let elem_types: CodegenResult<Vec<_>> = elems.iter().map(|e| e.to_llvm_type(lowering)).collect();
                let struct_type = lowering.context().struct_type(&elem_types?, false);
                Ok(struct_type.into())
            }
        }
    }
}

#[cfg(feature = "llvm")]
impl LlvmPointerType {
    fn to_llvm_type<'ctx>(&self, lowering: &LlvmTypeLowering<'ctx>) -> CodegenResult<BasicTypeEnum<'ctx>> {
        let pointee_type = self.pointee.to_llvm_type(lowering)?;
        let ptr_type = pointee_type.ptr_type(inkwell::AddressSpace::from(self.address_space));
        Ok(ptr_type.into())
    }
}

// Re-export core types
pub use crate::codegen::abi_core::*;