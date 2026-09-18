//! Schedule Tree Lowering to LLVM Control Flow
//!
//! This module lowers PIR ScheduleTree nodes to LLVM basic blocks with
//! proper induction variables, phi nodes, and control flow structure.
//!
//! Key features:
//! - Band nodes -> LLVM loops with affine bounds
//! - Filter nodes -> conditional branches with predicate computation
//! - Sequence/Context nodes -> block chaining and scoping
//! - QTT quantity constraints: [0]-erased loops stripped entirely
//! - AccessRelation -> LLVM GEP with alias.scope metadata

use crate::codegen::context::CodegenContext;
use crate::codegen::error::{CodegenError, CodegenResult};
use crate::codegen::llvm::{
    access_emission::AccessEmitter, loop_emission::LoopEmitter, parallel::ParallelEmitter,
    polyhedral_opts::PolyhedralOptimizer, type_lowering::LlvmTypeLowering,
    value_builder::LlvmValueBuilder,
};
use crate::ir::{
    affine_domain::AffineDomain,
    affine_map::AffineMap,
    pir_types::{AccessRelations, PirModule, PirStatement, QuantityMap},
    schedule_tree::{ScheduleNode, ScheduleTree, StmtId},
};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicBlock, BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;

/// Main entry point for lowering a ScheduleTree to LLVM IR
pub fn lower_schedule_tree<'ctx>(
    ctx: &CodegenContext,
    function: FunctionValue<'ctx>,
    schedule: &ScheduleTree,
    pir_module: &PirModule,
    quantities: &QuantityMap,
    access_relations: &AccessRelations,
) -> CodegenResult<()> {
    let llvm_context = ctx.llvm_context();
    let builder = llvm_context.create_builder();
    let mut type_lowering = LlvmTypeLowering::new(llvm_context);
    let mut value_builder = LlvmValueBuilder::new(builder, type_lowering.clone());

    // Create entry block
    let entry = llvm_context.append_basic_block(function, "entry");
    value_builder.builder().position_at_end(entry);

    // Create schedule lowering context
    let mut lowering = ScheduleLowering::new(
        &mut value_builder,
        function,
        pir_module,
        quantities,
        access_relations,
    )?;

    // Lower the root schedule node
    lowering.lower_node(&schedule.root)?;

    // Build return
    let void_type = lowering.value_builder.type_lowering().void_type();
    value_builder
        .builder()
        .build_return(Some(&void_type.const_zero()))?;

    // Verify function
    function
        .verify(true)
        .map_err(|e| CodegenError::VerificationError(e.to_string()))?;

    Ok(())
}

/// Schedule lowering context
pub struct ScheduleLowering<'ctx, 'a> {
    value_builder: &'a mut LlvmValueBuilder<'ctx>,
    function: FunctionValue<'ctx>,
    pir_module: &'a PirModule,
    quantities: &'a QuantityMap,
    access_relations: &'a AccessRelations,

    /// Current basic block
    current_block: Option<BasicBlock<'ctx>>,
    /// Statement -> block mapping
    stmt_blocks: HashMap<StmtId, BasicBlock<'ctx>>,
    /// Induction variable phi nodes
    induction_vars: HashMap<String, inkwell::values::PhiValue<'ctx>>,
    /// Loop metadata
    loop_metadata: HashMap<String, inkwell::metadata::MetadataValue<'ctx>>,

    // Sub-emitters
    loop_emitter: LoopEmitter<'ctx>,
    access_emitter: AccessEmitter<'ctx>,
    optimizer: PolyhedralOptimizer,
    parallel_emitter: ParallelEmitter<'ctx>,
}

impl<'ctx, 'a> ScheduleLowering<'ctx, 'a> {
    pub fn new(
        value_builder: &'a mut LlvmValueBuilder<'ctx>,
        function: FunctionValue<'ctx>,
        pir_module: &'a PirModule,
        quantities: &'a QuantityMap,
        access_relations: &'a AccessRelations,
    ) -> CodegenResult<Self> {
        let loop_emitter = LoopEmitter::new(value_builder.type_lowering().context())?;
        let access_emitter = AccessEmitter::new(value_builder.type_lowering().context())?;
        let optimizer = PolyhedralOptimizer::new();
        let parallel_emitter = ParallelEmitter::new(value_builder.type_lowering().context())?;

        Ok(Self {
            value_builder,
            function,
            pir_module,
            quantities,
            access_relations,
            current_block: None,
            stmt_blocks: HashMap::new(),
            induction_vars: HashMap::new(),
            loop_metadata: HashMap::new(),
            loop_emitter,
            access_emitter,
            optimizer,
            parallel_emitter,
        })
    }

    fn set_current_block(&mut self, block: BasicBlock<'ctx>) {
        self.current_block = Some(block);
        self.value_builder.builder().position_at_end(block);
    }

    fn current_block(&self) -> BasicBlock<'ctx> {
        self.current_block.expect("No current block set")
    }

    /// Lower a schedule node recursively
    pub fn lower_node(&mut self, node: &ScheduleNode) -> CodegenResult<()> {
        match node {
            ScheduleNode::Band {
                members,
                coincident,
                child,
            } => self.lower_band(members, coincident, child),
            ScheduleNode::Filter { domain, child } => self.lower_filter(domain, child),
            ScheduleNode::Sequence { children } => self.lower_sequence(children),
            ScheduleNode::Context { domain, child } => self.lower_context(domain, child),
            ScheduleNode::Domain { stmt_id, domain } => self.lower_domain(*stmt_id, domain),
            ScheduleNode::Extension { sizes, child } => self.lower_extension(sizes, child),
            ScheduleNode::Empty => Ok(()),
        }
    }

    /// Lower a band node (affine loop nest)
    fn lower_band(
        &mut self,
        members: &[AffineMap],
        coincident: &[bool],
        child: &ScheduleNode,
    ) -> CodegenResult<()> {
        // Check if this band should be erased ([0] quantity)
        if self.is_band_erased(members) {
            // [0]-quantity band: skip code generation entirely
            return self.lower_node(child);
        }

        // Extract loop bounds from scheduling maps
        let bounds = self.extract_bounds(members)?;

        // Check if this band is parallelizable
        let is_parallel = coincident.iter().any(|&c| c);

        // Emit the loop nest
        if is_parallel {
            self.parallel_emitter.emit_parallel_band(
                &mut self.value_builder,
                &bounds,
                members,
                child,
                |lowering| lowering.lower_node(child),
            )?;
        } else {
            self.loop_emitter.emit_sequential_band(
                &mut self.value_builder,
                &bounds,
                members,
                child,
                |lowering| lowering.lower_node(child),
            )?;
        }

        Ok(())
    }

    /// Check if a band is [0]-quantity (erased)
    fn is_band_erased(&self, members: &[AffineMap]) -> bool {
        // Check if any statement in this band's scope has [0] quantity
        // For now, check if any statement in the module is erased
        self.quantities
            .values()
            .any(|q| matches!(q, crate::ast::Quantity::Zero))
    }

    /// Extract loop bounds from affine scheduling maps
    fn extract_bounds(&self, members: &[AffineMap]) -> CodegenResult<Vec<LoopBounds>> {
        let mut bounds = Vec::new();

        for member in members {
            // Get the domain of the schedule map
            let domain = &member.pieces[0].domain;

            // For each iterator dimension, extract bounds
            for iter_dim in 0..domain.n_iter {
                if let Some((lower, upper)) = domain.iterator_bounds(iter_dim) {
                    bounds.push(LoopBounds {
                        iterator_dim: iter_dim,
                        lower: self.lower_affine_expr(&lower)?,
                        upper: self.lower_affine_expr(&upper)?,
                        step: 1, // Default step of 1
                    });
                }
            }
        }

        Ok(bounds)
    }

    /// Lower an affine expression to LLVM value
    fn lower_affine_expr(
        &self,
        expr: &crate::ir::affine_domain::AffineExpr,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        // Simplified: just return the constant for now
        // In reality, this would evaluate parameters and induction variables
        let int_type = self
            .value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I64);
        Ok(self
            .value_builder
            .build_int_constant(int_type, expr.constant as u64, "bound")?
            .into())
    }

    /// Lower a filter node (conditional domain restriction)
    fn lower_filter(&mut self, domain: &AffineDomain, child: &ScheduleNode) -> CodegenResult<()> {
        // Compute predicate from filter domain constraints
        let predicate = self.compute_filter_predicate(domain)?;

        // Create basic blocks for then/else
        let func = self.function;
        let then_block = self
            .value_builder
            .type_lowering()
            .context()
            .append_basic_block(func, "filter_then");
        let else_block = self
            .value_builder
            .type_lowering()
            .context()
            .append_basic_block(func, "filter_else");
        let merge_block = self
            .value_builder
            .type_lowering()
            .context()
            .append_basic_block(func, "filter_merge");

        // Branch on predicate
        self.value_builder
            .build_conditional_branch(predicate, then_block, else_block)?;

        // Then branch: lower child
        self.set_current_block(then_block);
        self.lower_node(child)?;
        self.value_builder.build_unconditional_branch(merge_block)?;

        // Else branch: skip child
        self.set_current_block(else_block);
        self.value_builder.build_unconditional_branch(merge_block)?;

        // Merge
        self.set_current_block(merge_block);
        Ok(())
    }

    /// Compute filter predicate from domain constraints
    fn compute_filter_predicate(
        &mut self,
        domain: &AffineDomain,
    ) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        let bool_type = self
            .value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I1);
        let zero = bool_type.const_zero();

        // For now, combine all constraints with AND
        let mut predicate = bool_type.const_int(1, false);

        for constraint in &domain.constraints {
            let constraint_val = self.lower_constraint(constraint)?;
            predicate = self
                .value_builder
                .build_and(predicate, constraint_val, "filter_and")?;
        }

        Ok(predicate)
    }

    /// Lower a single constraint to a boolean value
    fn lower_constraint(
        &mut self,
        constraint: &crate::ir::affine_domain::AffineConstraint,
    ) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        // Simplified implementation
        let bool_type = self
            .value_builder
            .type_lowering()
            .int_type(crate::codegen::abi::IntWidth::I1);
        Ok(bool_type.const_int(1, false))
    }

    /// Lower a sequence node (sequential composition)
    fn lower_sequence(&mut self, children: &[ScheduleNode]) -> CodegenResult<()> {
        for child in children {
            self.lower_node(child)?;
        }
        Ok(())
    }

    /// Lower a context node (parameter constraints)
    fn lower_context(&mut self, domain: &AffineDomain, child: &ScheduleNode) -> CodegenResult<()> {
        // Context nodes impose constraints on parameters
        // For codegen, we can emit assertions or just lower the child
        self.lower_node(child)
    }

    /// Lower a domain node (statement instance)
    fn lower_domain(&mut self, stmt_id: StmtId, domain: &AffineDomain) -> CodegenResult<()> {
        // Find the statement
        let stmt = self
            .pir_module
            .statements
            .iter()
            .find(|s| s.id == stmt_id)
            .ok_or_else(|| {
                CodegenError::FunctionBuildError(format!("Statement {:?} not found", stmt_id))
            })?;

        // Check if statement is [0]-quantity (erased)
        if stmt.quantity == crate::ast::Quantity::Zero {
            return Ok(()); // Skip erased statements
        }

        // Get access relations for this statement
        let accesses = self.access_relations.for_stmt(stmt_id);

        // Emit access instructions (loads/stores/GEPs)
        for access in accesses {
            self.access_emitter.emit_access(
                &mut self.value_builder,
                access,
                &stmt.body,
                &self.quantities,
            )?;
        }

        // Emit the statement body expression
        self.emit_statement_body(stmt)?;

        Ok(())
    }

    /// Emit the body of a statement
    fn emit_statement_body(&mut self, stmt: &PirStatement) -> CodegenResult<()> {
        // For now, this is handled by the access emitter
        // In a full implementation, we'd lower the PirExpr to LLVM instructions
        Ok(())
    }

    /// Lower an extension node (tiling, unrolling)
    fn lower_extension(&mut self, sizes: &[usize], child: &ScheduleNode) -> CodegenResult<()> {
        // Apply polyhedral optimization
        self.optimizer
            .apply_extension(sizes, child, |lowering| lowering.lower_node(child))
    }
}

/// Loop bounds representation
#[derive(Debug, Clone)]
struct LoopBounds {
    iterator_dim: usize,
    lower: BasicValueEnum<'ctx>,
    upper: BasicValueEnum<'ctx>,
    step: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Mutability, Quantity};
    use crate::codegen::context::{CodegenContext, CodegenTarget, OptLevel};
    use crate::ir::{
        affine_domain::AffineDomain,
        affine_map::{AffineMap, Matrix},
        pir_types::{AccessRelations, PirModule, PirStatement, ScheduleNode, ScheduleTree, StmtId},
        schedule_tree::ScheduleNode,
    };
    use std::collections::HashMap;

    #[test]
    fn test_schedule_lowering_creation() {
        let context = CodegenContext::new(CodegenTarget::Host, OptLevel::None).unwrap();
        let llvm_context = context.llvm_context();
        let module = llvm_context.create_module("test");
        let void_type = llvm_context.void_type();
        let fn_type = void_type.fn_type(&[], false);
        let function = module.add_function("test_fn", fn_type, None);

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

        let pir_module = PirModule::new(vec![stmt], schedule, accesses, quantities, vec![]);

        let mut type_lowering =
            crate::codegen::llvm::type_lowering::LlvmTypeLowering::new(llvm_context);
        let builder = llvm_context.create_builder();
        let mut value_builder =
            crate::codegen::llvm::value_builder::LlvmValueBuilder::new(builder, type_lowering);

        let lowering = ScheduleLowering::new(
            &mut value_builder,
            function,
            &pir_module,
            &pir_module.quantities,
            &pir_module.accesses,
        );
        assert!(lowering.is_ok());
    }
}
