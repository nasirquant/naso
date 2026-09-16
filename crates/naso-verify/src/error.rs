//! Error types for the verification engine.

use naso_compiler::ast::Span;
use thiserror::Error;

/// Result type for verification operations.
pub type VerifyResult<T> = Result<T, VerifyError>;

/// Errors that can occur during verification.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Error during AST to SMT-LIB2 lowering.
    #[error("Lowering error: {0}")]
    Lowering(#[from] LoweringError),

    /// Error during SMT solving.
    #[error("Solver error: {0}")]
    Solver(#[from] SolverError),

    /// Error during prover execution.
    #[error("Prover error: {0}")]
    Prover(#[from] ProverError),

    /// Error serializing/deserializing.
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Cache error.
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    Config(String),

    /// Internal invariant violation.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Errors during AST to SMT-LIB2 lowering.
#[derive(Debug, Error)]
pub enum LoweringError {
    /// Unsupported AST node encountered.
    #[error("Unsupported AST node at {span:?}: {kind}")]
    UnsupportedNode { span: Span, kind: String },

    /// Quantity annotation missing or invalid.
    #[error("Invalid quantity at {span:?}: {msg}")]
    InvalidQuantity { span: Span, msg: String },

    /// MVS (Mutable Value Semantics) violation in lowering.
    #[error("MVS lowering error at {span:?}: {msg}")]
    MvsError { span: Span, msg: String },

    /// Quantum operation not supported in verification.
    #[error("Quantum lowering error at {span:?}: {msg}")]
    QuantumError { span: Span, msg: String },

    /// Polyhedral construct not supported.
    #[error("Polyhedral lowering error at {span:?}: {msg}")]
    PolyhedralError { span: Span, msg: String },

    /// SMT-LIB2 generation error.
    #[error("SMT-LIB2 generation error: {0}")]
    SmtlibGen(String),

    /// Type mismatch during lowering.
    #[error("Type mismatch at {span:?}: expected {expected}, found {found}")]
    TypeMismatch {
        span: Span,
        expected: String,
        found: String,
    },

    /// Unbound variable or symbol.
    #[error("Unbound symbol at {span:?}: {name}")]
    UnboundSymbol { span: Span, name: String },

    /// Verification condition too complex (resource limit).
    #[error("VC complexity exceeded at {span:?}: {details}")]
    ComplexityExceeded { span: Span, details: String },
}

/// Errors during SMT solving.
#[derive(Debug, Error)]
pub enum SolverError {
    /// Z3 initialization failed.
    #[error("Z3 initialization failed: {0}")]
    InitFailed(String),

    /// Z3 context creation failed.
    #[error("Z3 context creation failed: {0}")]
    ContextFailed(String),

    /// SMT-LIB2 parsing failed.
    #[error("SMT-LIB2 parse error: {0}")]
    ParseError(String),

    /// Solver timeout.
    #[error("Solver timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Solver out of memory.
    #[error("Solver out of memory (limit: {limit_mb} MB)")]
    OutOfMemory { limit_mb: usize },

    /// Solver returned unknown (incomplete theory).
    #[error("Solver returned unknown: {reason}")]
    Unknown { reason: String },

    /// Solver error from Z3.
    #[error("Z3 error: {0}")]
    Z3Error(String),

    /// Model extraction failed.
    #[error("Model extraction failed: {0}")]
    ModelExtractionFailed(String),

    /// Unsat core extraction failed.
    #[error("Unsat core extraction failed: {0}")]
    UnsatCoreExtractionFailed(String),

    /// Incremental solving error (push/pop mismatch).
    #[error("Incremental solving error: {0}")]
    IncrementalError(String),
}

/// Errors during prover execution.
#[derive(Debug, Error)]
pub enum ProverError {
    /// CFG construction failed.
    #[error("CFG construction failed: {0}")]
    CfgConstructionFailed(String),

    /// Invariant generation failed.
    #[error("Invariant generation failed for {function}: {reason}")]
    InvariantGenerationFailed { function: String, reason: String },

    /// Induction hypothesis too weak.
    #[error("Induction hypothesis too weak for loop at {span:?}")]
    WeakInductionHypothesis { span: Span },

    /// Quantum circuit too large for symbolic reasoning.
    #[error("Quantum circuit too large ({num_qubits} qubits, {num_gates} gates)")]
    CircuitTooLarge { num_qubits: usize, num_gates: usize },

    /// Uncomputation proof failed with counterexample.
    #[error("Uncomputation proof failed: {counterexample}")]
    UncomputationFailed { counterexample: String },

    /// Linearity proof failed with leak path.
    #[error("Linearity proof failed: {leak_path}")]
    LinearityFailed { leak_path: String },

    /// Prover timeout.
    #[error("Prover timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

/// Cache errors.
#[derive(Debug, Error)]
pub enum CacheError {
    /// Cache directory not accessible.
    #[error("Cache directory not accessible: {path}")]
    DirNotAccessible { path: String },

    /// Cache entry corrupted.
    #[error("Cache entry corrupted: {key}")]
    EntryCorrupted { key: String },

    /// Cache key collision.
    #[error("Cache key collision: {key}")]
    KeyCollision { key: String },

    /// Cache serialization failed.
    #[error("Cache serialization failed: {0}")]
    SerializationFailed(String),

    /// Cache deserialization failed.
    #[error("Cache deserialization failed: {0}")]
    DeserializationFailed(String),
}

impl VerifyError {
    /// Check if this is a timeout error (any kind).
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            VerifyError::Solver(SolverError::Timeout { .. })
                | VerifyError::Prover(ProverError::Timeout { .. })
        )
    }

    /// Check if this is a resource exhaustion error.
    pub fn is_resource_exhausted(&self) -> bool {
        matches!(
            self,
            VerifyError::Solver(SolverError::OutOfMemory { .. })
                | VerifyError::Lowering(LoweringError::ComplexityExceeded { .. })
        )
    }

    /// Get a user-friendly error code.
    pub fn error_code(&self) -> &'static str {
        match self {
            VerifyError::Lowering(_) => "NASO-VERIFY-LOWER",
            VerifyError::Solver(SolverError::Timeout { .. }) => "NASO-VERIFY-TIMEOUT",
            VerifyError::Solver(SolverError::OutOfMemory { .. }) => "NASO-VERIFY-OOM",
            VerifyError::Solver(_) => "NASO-VERIFY-SOLVER",
            VerifyError::Prover(ProverError::UncomputationFailed { .. }) => "NASO-VERIFY-UNC",
            VerifyError::Prover(ProverError::LinearityFailed { .. }) => "NASO-VERIFY-LIN",
            VerifyError::Prover(_) => "NASO-VERIFY-PROVER",
            VerifyError::Cache(_) => "NASO-VERIFY-CACHE",
            VerifyError::Config(_) => "NASO-VERIFY-CONFIG",
            VerifyError::Serde(_) => "NASO-VERIFY-SERDE",
            VerifyError::Io(_) => "NASO-VERIFY-IO",
            VerifyError::Internal(_) => "NASO-VERIFY-INTERNAL",
        }
    }
}
