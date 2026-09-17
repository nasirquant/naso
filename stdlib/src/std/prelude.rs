#![allow(unused)]

// Standard library prelude - re-exports common stdlib items
pub use crate::std::alloc::*;
pub use crate::std::quantum::*;
pub use crate::std::sys::*;
pub use crate::std::tensor::*;

// Define the Qubit type as an opaque pointer for now
// In the future, we may want to use the compiler's definition
pub type Qubit = *mut u8;
