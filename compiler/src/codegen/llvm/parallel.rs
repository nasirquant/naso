//! Parallel Emission for LLVM
//!
//! Emits parallel loop constructs with LLVM metadata for OpenMP
//! and automatic parallelization.

use crate::codegen::error::CodegenResult;
use crate::codegen::llvm::value_builder::LlvmValueBuilder;
use crate::ir::affine_map::AffineMap;
use inkwell::IntPredicate;
use inkwell::basic_block::BasicBlock;
use inkwell::values::{BasicValueEnum, FunctionValue};

/// Parallel emitter for parallel bands
pub struct ParallelEmitter<'ctx> {
    context: &'ctx inkwell::context::Context,
}

impl<'ctx> ParallelEmitter<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> CodegenResult<Self> {
        Ok(Self { context })
    }

    /// Emit a parallel band with llvm.loop.parallel_accesses metadata
    pub fn emit_parallel_band<F>(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        bounds: &[LoopBounds<'ctx>],
        _members: &[AffineMap],
        _child: &crate::ir::schedule_tree::ScheduleNode,
        mut lower_child: F,
    ) -> CodegenResult<()>
    where
        F: FnMut(&mut dyn ScheduleLoweringLike<'ctx>) -> CodegenResult<()>,
    {
        // Emit each loop with parallel metadata
        for bound in bounds {
            self.emit_parallel_loop(value_builder, bound, |vb| {
                lower_child(&mut MockLowering { value_builder: vb })
            })?;
        }
        Ok(())
    }

    /// Emit a single parallel loop
    fn emit_parallel_loop<F>(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        bounds: &LoopBounds<'ctx>,
        mut body_builder: F,
    ) -> CodegenResult<()>
    where
        F: FnMut(&mut LlvmValueBuilder<'ctx>) -> CodegenResult<()>,
    {
        let func = value_builder
            .builder()
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        // Create loop blocks (same structure as sequential but with metadata)
        let preheader = self.context.append_basic_block(func, "par_loop_preheader");
        let header = self.context.append_basic_block(func, "par_loop_header");
        let body = self.context.append_basic_block(func, "par_loop_body");
        let latch = self.context.append_basic_block(func, "par_loop_latch");
        let exit = self.context.append_basic_block(func, "par_loop_exit");

        // Branch to preheader
        value_builder.build_unconditional_branch(preheader)?;

        // Preheader: initialize induction variable
        value_builder.builder().position_at_end(preheader);
        let int_type = value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64);
        let init_val = value_builder.build_int_constant(
            int_type,
            bounds.lower.into_int_value().get_zero_extended_constant() as u64,
            "iv_init",
        )?;
        let iv_alloca = value_builder.build_alloca(int_type, "iv")?;
        value_builder.build_store(iv_alloca, init_val.into())?;
        value_builder.build_unconditional_branch(header)?;

        // Header: phi node for induction variable
        value_builder.builder().position_at_end(header);
        let phi = value_builder.build_phi(int_type.into(), "iv_phi")?;
        phi.add_incoming(&[(init_val.into(), preheader)]);

        // Load current induction variable value
        let iv_val = value_builder
            .build_load(iv_alloca, "iv_val")?
            .into_int_value();

        // Compare with upper bound
        let upper_val = bounds.upper.into_int_value();
        let cond =
            value_builder.build_int_compare(IntPredicate::SLT, iv_val, upper_val, "loop_cond")?;

        value_builder.build_conditional_branch(cond, body, exit)?;

        // Body
        value_builder.builder().position_at_end(body);

        // Add parallel metadata to the loop
        self.add_parallel_metadata(value_builder, header)?;

        body_builder(value_builder)?;
        value_builder.build_unconditional_branch(latch)?;

        // Latch: increment induction variable
        value_builder.builder().position_at_end(latch);
        let step_val = value_builder.build_int_constant(int_type, bounds.step as u64, "iv_step")?;
        let next_iv = value_builder.build_int_add(iv_val, step_val, "iv_next")?;
        value_builder.build_store(iv_alloca, next_iv.into())?;

        // Add incoming to phi
        phi.add_incoming(&[(next_iv.into(), latch)]);
        value_builder.build_unconditional_branch(header)?;

        // Exit
        value_builder.builder().position_at_end(exit);

        Ok(())
    }

    /// Add LLVM parallel loop metadata
    fn add_parallel_metadata(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        header: BasicBlock<'ctx>,
    ) -> CodegenResult<()> {
        // Add llvm.loop.parallel_accesses metadata to the loop
        // This tells LLVM that iterations of this loop can be executed in parallel
        let module = value_builder.builder().get_module().unwrap();
        let context = value_builder.type_lowering().context();

        // Create metadata node for parallel loop
        let parallel_md = context.create_string_metadata("llvm.loop.parallel_accesses");
        let md_node = context.create_metadata_node(&[parallel_md.into()]);

        // Attach to the loop header's terminator instruction
        // Note: In real implementation, we'd attach to the branch instruction in the latch
        // For now, we add it as function-level metadata
        module.add_metadata("llvm.loop.parallel_accesses", &md_node);

        // Also add OpenMP-compatible metadata
        let omp_md = context.create_string_metadata("omp parallel for");
        let omp_node = context.create_metadata_node(&[omp_md.into()]);
        module.add_metadata("llvm.loop.parallel_accesses", &omp_node);

        Ok(())
    }

    /// Emit OpenMP-style parallel region
    pub fn emit_parallel_region<F>(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        _num_threads: Option<BasicValueEnum<'ctx>>,
        mut region_builder: F,
    ) -> CodegenResult<()>
    where
        F: FnMut(&mut LlvmValueBuilder<'ctx>) -> CodegenResult<()>,
    {
        // Create parallel region entry/exit blocks
        let func = value_builder
            .builder()
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let entry = self.context.append_basic_block(func, "omp_parallel_entry");
        let exit = self.context.append_basic_block(func, "omp_parallel_exit");

        // Branch to parallel region
        value_builder.build_unconditional_branch(entry)?;

        value_builder.builder().position_at_end(entry);
        region_builder(value_builder)?;
        value_builder.build_unconditional_branch(exit)?;

        value_builder.builder().position_at_end(exit);
        Ok(())
    }

    /// Emit SIMD/vectorized loop
    pub fn emit_simd_loop<F>(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        bounds: &LoopBounds<'ctx>,
        simd_width: usize,
        mut body_builder: F,
    ) -> CodegenResult<()>
    where
        F: FnMut(&mut LlvmValueBuilder<'ctx>) -> CodegenResult<()>,
    {
        // Similar to parallel loop but with SIMD metadata
        let func = value_builder
            .builder()
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        let preheader = self.context.append_basic_block(func, "simd_loop_preheader");
        let header = self.context.append_basic_block(func, "simd_loop_header");
        let body = self.context.append_basic_block(func, "simd_loop_body");
        let latch = self.context.append_basic_block(func, "simd_loop_latch");
        let exit = self.context.append_basic_block(func, "simd_loop_exit");

        value_builder.build_unconditional_branch(preheader)?;

        value_builder.builder().position_at_end(preheader);
        let int_type = value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64);
        let init_val = value_builder.build_int_constant(
            int_type,
            bounds.lower.into_int_value().get_zero_extended_constant() as u64,
            "iv_init",
        )?;
        let iv_alloca = value_builder.build_alloca(int_type, "iv")?;
        value_builder.build_store(iv_alloca, init_val.into())?;
        value_builder.build_unconditional_branch(header)?;

        value_builder.builder().position_at_end(header);
        let phi = value_builder.build_phi(int_type.into(), "iv_phi")?;
        phi.add_incoming(&[(init_val.into(), preheader)]);

        let iv_val = value_builder
            .build_load(iv_alloca, "iv_val")?
            .into_int_value();
        let upper_val = bounds.upper.into_int_value();
        let cond =
            value_builder.build_int_compare(IntPredicate::SLT, iv_val, upper_val, "loop_cond")?;
        value_builder.build_conditional_branch(cond, body, exit)?;

        value_builder.builder().position_at_end(body);

        // Add SIMD metadata
        self.add_simd_metadata(value_builder, header, simd_width)?;

        body_builder(value_builder)?;
        value_builder.build_unconditional_branch(latch)?;

        value_builder.builder().position_at_end(latch);
        let step_val = value_builder.build_int_constant(
            int_type,
            (bounds.step * simd_width as i64) as u64,
            "iv_step",
        )?;
        let next_iv = value_builder.build_int_add(iv_val, step_val, "iv_next")?;
        value_builder.build_store(iv_alloca, next_iv.into())?;
        phi.add_incoming(&[(next_iv.into(), latch)]);
        value_builder.build_unconditional_branch(header)?;

        value_builder.builder().position_at_end(exit);
        Ok(())
    }

    /// Add SIMD metadata
    fn add_simd_metadata(
        &self,
        value_builder: &mut LlvmValueBuilder<'ctx>,
        _header: BasicBlock<'ctx>,
        width: usize,
    ) -> CodegenResult<()> {
        let module = value_builder.builder().get_module().unwrap();
        let context = value_builder.type_lowering().context();

        let simd_md =
            context.create_string_metadata(&format!("llvm.loop.vectorize.width {}", width));
        let md_node = context.create_metadata_node(&[simd_md.into()]);
        module.add_metadata("llvm.loop.vectorize", &md_node);

        Ok(())
    }
}

/// Loop bounds structure (shared with loop_emission)
#[derive(Debug, Clone)]
pub struct LoopBounds<'ctx> {
    pub iterator_dim: usize,
    pub lower: BasicValueEnum<'ctx>,
    pub upper: BasicValueEnum<'ctx>,
    pub step: i64,
}

/// Trait for schedule lowering to allow mocking
pub trait ScheduleLoweringLike<'ctx> {
    fn value_builder(&mut self) -> &mut LlvmValueBuilder<'ctx>;
}

struct MockLowering<'ctx, 'a> {
    value_builder: &'a mut LlvmValueBuilder<'ctx>,
}

impl<'ctx, 'a> ScheduleLoweringLike<'ctx> for MockLowering<'ctx, 'a> {
    fn value_builder(&mut self) -> &mut LlvmValueBuilder<'ctx> {
        self.value_builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::context::{CodegenContext, CodegenTarget, OptLevel};

    #[test]
    fn test_parallel_emitter_creation() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let emitter = ParallelEmitter::new(context.llvm_context());
        assert!(emitter.is_ok());
    }
}
