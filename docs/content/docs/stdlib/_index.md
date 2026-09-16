---
title: "Standard Library"
weight: 30
---

# Naso Standard Library

The Naso standard library (`std`) provides essential modules for systems programming, quantum computing, polyhedral compilation, formal verification, and memory management.

## Module Overview

| Module | Description | Key Types |
|--------|-------------|-----------|
| `std::quantum` | Quantum gates, qubit management, measurement | `Qubit`, `QubitArray[N]`, `QReg` |
| `std::poly` | Polyhedral loop transformations, affine maps | `PolyDomain`, `AffineMap`, `Schedule` |
| `std::smt` | Embedded Z3 SMT solver interface | `Solver`, `Expr`, `Model`, `Sort` |
| `std::mem` | Linear memory management, allocation | `LinearBox`, `LinearVec`, `qalloc`, `qfree` |
| `std::io` | Classical I/O, serialization | `stdin`, `stdout`, `serialize`, `deserialize` |
| `std::math` | Linear algebra, numerical routines | `Matrix`, `Vector`, `fft`, `linalg` |

---

## `std::quantum`

The quantum module provides types and operations for quantum computing with linear type guarantees.

### Types

| Type | Quantity | Description |
|------|----------|-------------|
| `Qubit` | `[1]` | Single linear qubit — must be consumed exactly once |
| `QubitArray[N]` | `[1]` | Fixed-size array of N linear qubits |
| `QReg[N]` | `[1]` | Quantum register with N qubits (alias for `QubitArray[N]`) |
| `Cbit` | `[*]` | Classical bit (measurement result) |
| `CReg[N]` | `[*]` | Classical register of N bits |

### Core Gates

| Function | Signature | Description |
|----------|-----------|-------------|
| `qalloc` | `fn() -> [1] Qubit` | Allocate a fresh qubit in \|0⟩ state |
| `qfree` | `fn(q: [1] Qubit)` | Deallocate qubit (must be in \|0⟩) |
| `hadamard` | `fn(inout q: [1] Qubit)` | Apply H gate: \|0⟩ → ( \|0⟩ + \|1⟩ )/√2 |
| `pauli_x` | `fn(inout q: [1] Qubit)` | Apply X gate: \|0⟩ ↔ \|1⟩ |
| `pauli_y` | `fn(inout q: [1] Qubit)` | Apply Y gate |
| `pauli_z` | `fn(inout q: [1] Qubit)` | Apply Z gate: phase flip |
| `rx` | `fn(inout q: [1] Qubit, theta: f64)` | Rotation around X axis |
| `ry` | `fn(inout q: [1] Qubit, theta: f64)` | Rotation around Y axis |
| `rz` | `fn(inout q: [1] Qubit, theta: f64)` | Rotation around Z axis |
| `cnot` | `fn(inout ctl: [1] Qubit, inout tgt: [1] Qubit)` | Controlled-NOT |
| `cz` | `fn(inout ctl: [1] Qubit, inout tgt: [1] Qubit)` | Controlled-Z |
| `swap` | `fn(inout a: [1] Qubit, inout b: [1] Qubit)` | SWAP gate |
| `measure` | `fn(inout q: [1] Qubit) -> [1] Cbit` | Measure in Z basis, consumes qubit |
| `measure_x` | `fn(inout q: [1] Qubit) -> [1] Cbit` | Measure in X basis |
| `reset` | `fn(inout q: [1] Qubit) -> [0] Proof` | Reset to \|0⟩ (requires proof of known state) |

### Example: Bell State Preparation

```naso
use std::quantum::{qalloc, qfree, hadamard, cnot, measure};

fn create_bell_pair() -> [2] Qubit {
    let q0 = qalloc();
    let q1 = qalloc();
    hadamard(inout q0);
    cnot(inout q0, inout q1);
    [q0, q1] // Return tuple of linear qubits
}

fn measure_bell(inout pair: [2] Qubit) -> [2] Cbit {
    let [q0, q1] = pair; // Destructure
    [measure(inout q0), measure(inout q1)]
}
```

### Example: GHZ State (N-qubit Entanglement)

```naso
fn ghz_state(n: usize) -> [N] Qubit
where [N] = [n] {
    let mut reg = QReg::new(n); // [1] QReg[N]
    hadamard(inout reg[0]);
    for i in 1..n {
        cnot(inout reg[0], inout reg[i]);
    }
    reg.into_array() // [1] QubitArray[N]
}
```

### Example: Quantum Teleportation Protocol

```naso
fn teleport(inout psi: [1] Qubit, inout alice: [1] Qubit, inout bob: [1] Qubit) 
    -> ([1] Cbit, [1] Cbit, [1] Qubit) {
    
    // Create Bell pair between Alice and Bob
    hadamard(inout alice);
    cnot(inout alice, inout bob);
    
    // Bell measurement on psi and Alice's qubit
    cnot(inout psi, inout alice);
    hadamard(inout psi);
    let m1 = measure(inout psi);    // Consumes psi
    let m2 = measure(inout alice);  // Consumes alice
    
    // Conditional corrections on Bob's qubit (classical control)
    // In QASM: if (m1) x bob; if (m2) z bob;
    // Here modeled as proof obligation:
    // m1=true → apply X to bob; m2=true → apply Z to bob
    
    (m1, m2, bob) // Return classical bits and Bob's qubit
}
```

---

## `std::poly`

The polyhedral module provides utilities for working with polyhedral loop transformations and static analysis.

### Types

| Type | Description |
|------|-------------|
| `PolyDomain` | Represents a polyhedral iteration domain (Z-polyhedron) |
| `AffineMap` | Represents an affine transformation: x ↦ Ax + b |
| `Schedule` | Time-space mapping for loop nests |
| `Dependence` | Data dependence relation between statements |

### Domain Construction

| Function | Signature | Description |
|----------|-----------|-------------|
| `domain` | `fn(bounds: &[Range]) -> PolyDomain` | Create domain from loop bounds |
| `intersect` | `fn(a: PolyDomain, b: PolyDomain) -> PolyDomain` | Domain intersection |
| `project` | `fn(dom: PolyDomain, dims: &[usize]) -> PolyDomain` | Project onto subset of dimensions |

### Transformations

| Function | Signature | Description |
|----------|-----------|-------------|
| `tile` | `fn(dom: PolyDomain, factors: &[usize]) -> PolyDomain` | Tile domain for cache blocking |
| `parallel` | `fn(dom: PolyDomain) -> PolyDomain` | Mark domain for parallel execution |
| `vectorize` | `fn(dom: PolyDomain, width: usize) -> PolyDomain` | Vectorize innermost loop |
| `skew` | `fn(dom: PolyDomain, factor: isize) -> PolyDomain` | Skew transformation for wavefront |
| `interchange` | `fn(dom: PolyDomain, i: usize, j: usize) -> PolyDomain` | Loop interchange |
| `fuse` | `fn(a: PolyDomain, b: PolyDomain) -> PolyDomain` | Loop fusion |

### Schedule Construction

```naso
use std::poly::{domain, tile, parallel, vectorize, Schedule};

fn build_schedule() -> Schedule {
    let dom = domain(&[0..N, 0..M, 0..P]);
    
    // Tile for L1 cache (32x32 tiles)
    let tiled = tile(dom, &[32, 32, 1]);
    
    // Parallelize outer tile loops
    let par = parallel(tiled);
    
    // Vectorize innermost loop (AVX-512: width=8 for f64)
    let vec = vectorize(par, 8);
    
    Schedule::from_domain(vec)
}
```

### Example: Tiled Matrix Multiplication with Schedule

```naso
use std::poly::{domain, tile, parallel, vectorize, Schedule};

fn matmul_tiled(A: [N][M] f64, B: [M][P] f64) -> [N][P] f64 {
    // Compiler uses this schedule for code generation
    #[schedule(build_schedule())]
    fn matmul_kernel(A: [N][M] f64, B: [M][P] f64) -> [N][P] f64 {
        let C = [[0.0; P]; N];
        for i in 0..N {
            for j in 0..P {
                for k in 0..M {
                    C[i][j] += A[i][k] * B[k][j];
                }
            }
        }
        C
    }
    matmul_kernel(A, B)
}
```

### Example: Stencil with Wavefront Parallelization

```naso
use std::poly::{domain, skew, parallel, Schedule};

fn stencil_3d_schedule() -> Schedule {
    let dom = domain(&[1..N-1, 1..M-1, 1..O-1]);
    
    // Skew i-j plane for wavefront parallelism
    let skewed = skew(dom, 1); // factor=1 for i+j = constant
    
    // Parallelize skewed dimension
    let par = parallel(skewed);
    
    Schedule::from_domain(par)
}
```

---

## `std::smt`

The SMT module provides an interface to the embedded Z3 solver for compile-time verification.

### Types

| Type | Description |
|------|-------------|
| `Solver` | Z3 solver context (linear — must be consumed) |
| `Expr` | Symbolic expression (quantifier-free) |
| `Sort` | SMT sort: `Bool`, `Int`, `Real`, `BitVec[N]`, `Array` |
| `Model` | Satisfying assignment (if `check` returns `Sat`) |
| `Proof` | Unsatisfiability proof (if `check` returns `Unsat`) |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `new_solver` | `fn() -> [1] Solver` | Create new Z3 context |
| `declare_const` | `fn(s: &mut Solver, name: &str, sort: Sort) -> Expr` | Declare uninterpreted constant |
| `assert` | `fn(s: &mut Solver, e: Expr)` | Assert constraint |
| `check` | `fn(s: &Solver) -> CheckResult` | Check satisfiability |
| `get_model` | `fn(s: &Solver) -> Option<Model>` | Get model if satisfiable |
| `get_proof` | `fn(s: &Solver) -> Option<Proof>` | Get unsat proof |
| `expr_from` | `fn<T: IntoExpr>(v: T) -> Expr` | Convert Naso value to SMT expr |

### CheckResult Enum

```naso
enum CheckResult {
    Sat,      // Constraints satisfiable
    Unsat,    // Constraints unsatisfiable
    Unknown,  // Solver cannot determine (timeout/incomplete)
}
```

### Example: Array Bounds Verification

```naso
use std::smt::{new_solver, assert, check, expr_from, Sort};

fn safe_get(arr: [*] [N] f64, idx: usize) -> f64 {
    let mut solver = new_solver();
    let idx_e = expr_from(idx);
    let n_e = expr_from(N);
    
    // Prove 0 <= idx < N
    assert(&mut solver, idx_e >= expr_from(0));
    assert(&mut solver, idx_e < n_e);
    
    match check(&solver) {
        CheckResult::Sat => {
            // Bounds proven at compile time — no runtime check needed
            arr[idx]
        }
        CheckResult::Unsat => {
            compile_error!("Index out of bounds: solver proved idx >= N");
        }
        CheckResult::Unknown => {
            // Fallback: insert runtime check
            if idx < N { arr[idx] } else { panic!("Index out of bounds") }
        }
    }
}
```

### Example: Linear Arithmetic Verification

```naso
use std::smt::{new_solver, assert, check, declare_const, Sort, expr_from};

fn verify_invariant(x: i32, y: i32) -> [0] Proof {
    let mut solver = new_solver();
    
    // Declare symbolic versions
    let x_s = declare_const(&mut solver, "x", Sort::Int);
    let y_s = declare_const(&mut solver, "y", Sort::Int);
    
    // Precondition: x > 0 && y > 0
    assert(&mut solver, x_s > expr_from(0));
    assert(&mut solver, y_s > expr_from(0));
    
    // Operation: z = x + y
    let z_s = x_s + y_s;
    
    // Postcondition: z > x && z > y
    assert(&mut solver, z_s > x_s);
    assert(&mut solver, z_s > y_s);
    
    // This will be checked at compile time
    match check(&solver) {
        CheckResult::Sat => (), // Proven
        CheckResult::Unsat => compile_error!("Invariant violated"),
        CheckResult::Unknown => compile_error!("Verification incomplete"),
    }
}
```

### Example: Quantifier-Free Bitvector Reasoning

```naso
use std::smt::{new_solver, assert, check, declare_const, Sort, expr_from};

fn verify_overflow(a: u32, b: u32) -> [0] Proof {
    let mut solver = new_solver();
    let a_s = declare_const(&mut solver, "a", Sort::BitVec(32));
    let b_s = declare_const(&mut solver, "b", Sort::BitVec(32));
    
    // Check if a + b overflows (unsigned)
    let sum = a_s.bvadd(b_s);
    let overflow = sum.bvult(a_s); // sum < a iff overflow
    
    assert(&mut solver, overflow);
    
    // If SAT, there exist inputs causing overflow
    match check(&solver) {
        CheckResult::Sat => {
            compile_warn!("Potential unsigned overflow detected");
        }
        CheckResult::Unsat => {
            // Proven no overflow for any inputs
        }
        CheckResult::Unknown => {}
    }
}
```

---

## `std::mem`

Memory management with linear types and quantum allocation.

### Types

| Type | Description |
|------|-------------|
| `LinearBox<T>` | `[1]` owning pointer — unique ownership |
| `LinearVec<T>` | `[1]` owning vector — linear buffer |
| `SharedBox<T>` | `[*]` reference-counted (classical only) |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `alloc` | `fn<T>(val: T) -> [1] LinearBox<T>` | Allocate on heap with linear ownership |
| `dealloc` | `fn<T>(box: [1] LinearBox<T>)` | Deallocate (consumes box) |
| `qalloc` | `fn() -> [1] Qubit` | Quantum allocation (from `std::quantum`) |
| `qfree` | `fn(q: [1] Qubit)` | Quantum deallocation |
| `vec_from_raw` | `fn(ptr: *mut T, len: usize, cap: usize) -> [1] LinearVec<T>` | Construct from raw parts |
| `vec_into_raw` | `fn(vec: [1] LinearVec<T>) -> (*mut T, usize, usize)` | Destructure to raw parts |

### Example: Linear Buffer Management

```naso
use std::mem::{alloc, dealloc, LinearBox};

fn process_data(data: [1] LinearBox<[N] f64>) -> [1] LinearBox<[N] f64> {
    // Exclusive mutable access to buffer
    let mut buf = data; // Move ownership
    
    for i in 0..N {
        buf[i] = buf[i] * 2.0; // In-place mutation
    }
    
    buf // Return ownership
}

fn main() {
    let boxed = alloc([0.0; 1024]); // [1] LinearBox<[1024] f64>
    let result = process_data(boxed); // boxed moved, result returned
    dealloc(result); // Explicit deallocation (consumes result)
}
```

---

## `std::io`

Classical I/O and serialization for host interaction.

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `print` | `fn(s: &str)` | Print to stdout |
| `println` | `fn(s: &str)` | Print with newline |
| `read_line` | `fn() -> String` | Read line from stdin |
| `serialize` | `fn<T: Serialize>(val: &T) -> Vec<u8>` | Serialize to bytes |
| `deserialize` | `fn<T: Deserialize>(bytes: &[u8]) -> Result<T>` | Deserialize from bytes |

### Example: Host-Device Communication

```naso
use std::io::{println, serialize, deserialize};

#[derive(Serialize, Deserialize)]
struct KernelConfig {
    grid_size: usize,
    block_size: usize,
    precision: Precision,
}

fn launch_kernel(config: KernelConfig) {
    let bytes = serialize(&config);
    // Send to device driver...
    println!("Launching kernel: {:?}", config);
}
```

---

## `std::math`

Linear algebra and numerical routines optimized via polyhedral compilation.

### Types

| Type | Description |
|------|-------------|
| `Matrix<N, M>` | `[N][M] f64` — compile-time sized matrix |
| `Vector<N>` | `[N] f64` — compile-time sized vector |

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `matmul` | `fn(A: [N][M] f64, B: [M][P] f64) -> [N][P] f64` | Matrix multiplication |
| `transpose` | `fn(A: [N][M] f64) -> [M][N] f64` | Matrix transpose |
| `dot` | `fn(a: [N] f64, b: [N] f64) -> f64` | Dot product |
| `norm2` | `fn(v: [N] f64) -> f64` | L2 norm |
| `fft` | `fn(v: [N] Complex) -> [N] Complex` | Fast Fourier Transform (N power of 2) |
| `svd` | `fn(A: [N][M] f64) -> (U, S, Vt)` | Singular Value Decomposition |

### Example: FFT with Polyhedral Optimization

```naso
use std::math::fft;

fn frequency_analysis(signal: [1024] f64) -> [1024] f64 {
    // Convert to complex
    let mut complex: [1024] Complex = [Complex::new(0.0, 0.0); 1024];
    for i in 0..1024 {
        complex[i] = Complex::new(signal[i], 0.0);
    }
    
    // FFT with automatic polyhedral scheduling
    let spectrum = fft(complex);
    
    // Compute magnitude spectrum
    let mut mag: [1024] f64 = [0.0; 1024];
    for i in 0..1024 {
        mag[i] = spectrum[i].re * spectrum[i].re + spectrum[i].im * spectrum[i].im;
    }
    mag
}
```

---

## Prelude Imports

The following are automatically available in every Naso file:

```naso
// Quantities
[0], [1], [*], [N]

// Core types
bool, int, f64, f32, u8..u64, i8..i64, usize, isize
Vec<T>, Option<T>, Result<T, E>

// Quantum
Qubit, QReg, qalloc, qfree, hadamard, cnot, measure

// Memory
LinearBox, LinearVec, alloc, dealloc

// SMT
Solver, Expr, Sort, Model, new_solver, assert, check
```