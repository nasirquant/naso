//! Polyhedral Intermediate Representation (PIR) for Naso Compiler
//!
//! This module defines the core data structures for polyhedral compilation:
//! - Affine domains (iteration spaces with parameters)
//! - Affine maps (piecewise quasi-affine functions)
//! - Schedule trees (hierarchical loop schedules)
//! - Access relations (memory access patterns)
//! - PIR module container with quantity tracking

pub mod affine_domain;
pub mod affine_map;
pub mod schedule_tree;
pub mod access_relation;
pub mod pir_types;
pub mod pretty_print;
pub mod validate;

#[cfg(test)]
pub mod tests;

// Re-exports for convenience
pub use affine_domain::{AffineDomain, AffineConstraint, ConstraintType};
pub use affine_map::{AffineMap, AffineMapPiece, Matrix};
pub use schedule_tree::{ScheduleNode, ScheduleTree, StmtId, ScheduleValidationError};
pub use access_relation::{AccessRelation, AccessType, AccessRelations};
pub use pir_types::{PirModule, PirStatement, QuantityMap, PirExpr, ValidationError, BinaryOp, UnaryOp};
pub use pretty_print::{pir_to_string, pir_to_json, format_golden_fixture};
pub use validate::{validate_pir, ScheduleValidationReport, validate_domain, validate_map, validate_schedule_detailed, validate_accesses_for_dependence};