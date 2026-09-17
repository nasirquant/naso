//! Diagnostic Code Constants
//!
//! Standardized diagnostic codes for Naso Language Server.
//! Each code follows the pattern: NASO-<CATEGORY>-<NUMBER>
//! Categories: LIN (Linearity), ERA (Erasure), MVS (Mutable Value Semantics), UNC (Uncomputation)

/// Linearity diagnostic codes
pub mod lin {
    /// Linear variable used more than once
    pub const DOUBLE_USE: &str = "NASO-LIN-001";

    /// Linear variable not consumed (unused)
    pub const UNUSED: &str = "NASO-LIN-002";

    /// Linear variable implicitly dropped without explicit consumption
    pub const IMPLICIT_DROP: &str = "NASO-LIN-003";

    /// Use of moved value (consumed then used again)
    pub const USE_OF_MOVED: &str = "NASO-LIN-004";
}

/// Erasure diagnostic codes
pub mod era {
    /// Proof-only [0] quantity value retained at runtime
    pub const RETAINED_AT_RUNTIME: &str = "NASO-ERA-001";

    /// Non-erased proof term in compiled output
    pub const NON_ERASED_PROOF: &str = "NASO-ERA-002";
}

/// Mutable Value Semantics diagnostic codes
pub mod mvs {
    /// Inout parameter aliases with existing inout borrow
    pub const INOUT_ALIASING: &str = "NASO-MVS-001";

    /// Inout parameter escapes its scope
    pub const INOUT_ESCAPE: &str = "NASO-MVS-002";

    /// Inout requires unique ownership (quantity 1)
    pub const INOUT_REQUIRES_UNIQUE: &str = "NASO-MVS-003";
}

/// Uncomputation diagnostic codes
pub mod unc {
    /// Missing uncomputation step for temporary variable
    pub const MISSING_UNCOMPUTE: &str = "NASO-UNC-001";

    /// Cyclic uncomputation dependency detected
    pub const CYCLIC_UNCOMPUTE: &str = "NASO-UNC-002";

    /// Non-invertible temporary value cannot be uncomputed
    pub const NON_INVERTIBLE_TEMP: &str = "NASO-UNC-003";
}

/// All diagnostic codes as a flat list for registration
pub const ALL_CODES: &[&str] = &[
    lin::DOUBLE_USE,
    lin::UNUSED,
    lin::IMPLICIT_DROP,
    lin::USE_OF_MOVED,
    era::RETAINED_AT_RUNTIME,
    era::NON_ERASED_PROOF,
    mvs::INOUT_ALIASING,
    mvs::INOUT_ESCAPE,
    mvs::INOUT_REQUIRES_UNIQUE,
    unc::MISSING_UNCOMPUTE,
    unc::CYCLIC_UNCOMPUTE,
    unc::NON_INVERTIBLE_TEMP,
];
