---
title: "Formal Verification (naso-verify)"
weight: 40
---

# Formal Verification with naso-verify

Naso includes an integrated SMT-based verifier (`naso-verify`) that checks safety properties at compile time, eliminating entire classes of bugs before code generation.

## Guarantees

The verifier provides the following guarantees:

### 1. Non-Aliasing (Linear Types)
- Ensures that `[1]` (linear) resources have at most one reference at any time.
- Prevents use-after-free, double-free, and data races at compile time.
- Example: A linear buffer can only be owned by one variable; attempting to copy it triggers a compile-time error.

### 2. Quantum State Uncomputation (Bennett Compliance)
- Verifies that all temporary quantum states are uncomputed before scope exit.
- Ensures that ancilla qubits are returned to the |0⟩ state, allowing safe reuse or deallocation.
- Critical for reversible computing and quantum error correction.

### 3. Static Array Bounds
- Proves that all array accesses are within provable bounds at compile time.
- Eliminates buffer overflows without runtime checks.
- Works with symbolic indices and polyhedral loop domains.

### 4. Resource Bounds (QTT)
- Checks that `[0]` variables are erased before codegen (zero runtime cost).
- Ensures `[1]` variables are consumed exactly once (no leaks, no drops).
- Validates that `[N]` quantities respect their static bounds.

## How It Works

`naso-verify` is invoked automatically during the build process for functions annotated with `verify` or when running `naso verify <function>`.

It translates the function's preconditions, postconditions, and body into SMT queries (using the Z3 solver) and checks:
- Memory safety (no null/dangling pointers)
- Type safety (correct use of quantities)
- Quantum state consistency (uncomputation of temporaries)
- Assertions and contracts specified in the code

## SMT-LIB2 Translation Examples

### Basic Assertion Encoding

For a Naso function:
```naso
fn divide(a: f64, b: f64) -> f64 {
    assert(b != 0.0);
    a / b
}
```

**Generated SMT-LIB2:**
```lisp
(declare-fun a () Real)
(declare-fun b () Real)
(declare-fun result () Real)
(assert (= result (/ a b)))
(assert (not (= b 0))) ; from assert(b != 0.0)
(check-sat)
(get-model)
```

### Linear Variable Usage Tracking

For linear variable consumption:
```naso
fn process(buf: [1] Vec<[N] u8>) {
    let len = buf.len(); // First use
    buf.clear();         // Second use - ERROR
}
```

**Generated SMT-LIB2 (simplified):**
```lisp
(declare-fun buf_init () Array)
(declare-fun buf_after_len () Array)
(declare-fun buf_after_clear () Array)

; Track linear state transitions
(assert (= buf_after_len buf_init)) ; len() doesn't consume
(assert (= buf_after_clear (as const ([*]) Array))) ; clear() consumes

; Check that final state is consumed exactly once
(assert (distinct buf_after_clear buf_init)) ; Should be distinct
(assert (= buf_after_clear (as const ([*]) Array))) ; Final state is consumed

(check-sat) ; UNSAT indicates linear usage violation
```

### Quantum State Tracking

For entanglement verification:
```naso
fn entangle_check() {
    let q0 = qalloc();
    let q1 = qalloc();
    cnot(inout q0, inout q1); // Creates entanglement
    // q0, q1 must be uncomputed together
}
```

**Generated SMT-LIB2:**
```lisp
; Quantum state as density matrix (simplified)
(declare-fun rho00 () Real) ; |00><00|
(declare-fun rho01 () Real) ; |00><01|
(declare-fun rho10 () Real) ; |01><00|
(declare-fun rho11 () Real) ; |01><01|

; Initial state: |00>
(assert (= rho00 1.0))
(assert (= rho01 0.0))
(assert (= rho10 0.0))
(assert (= rho11 0.0))

; After CNOT: (|00> + |11>)/√2
; rho00 = 0.5, rho11 = 0.5, rho01=rho10=0
(assert (= rho00 0.5))
(assert (= rho11 0.5))
(assert (= rho01 0.0))
(assert (= rho10 0.0))

; Uncompute requirement: final state must be |00>
(assert (or (= rho00 0.0) (= rho11 0.0))) ; Violation if entangled remains
(check-sat) ; UNSAT means cannot disentangle - ERROR
```

## Complete Error Code Matrix

### Linear Type Errors (NASO-LIN-*)
| Code | Message | Description |
|------|---------|-------------|
| **NASO-LIN-001** | Linear variable dropped without consumption | A `[1]` variable went out of scope without being used |
| **NASO-LIN-002** | Linear variable used more than once | A `[1]` variable was accessed twice without move/borrow |
| **NASO-LIN-003** | Linear variable consumed after move | Attempted to use a `[1]` variable after it was moved |
| **NASO-LIN-004** | Linear variable borrowed mutably and immutably simultaneously | Violates exclusive access for `inout` |
| **NASO-LIN-005** | Linear return value ignored | Function returning `[1]` T but caller didn't use result |

### Mutable Value Semantics Errors (NASO-MVS-*)
| Code | Message | Description |
|------|---------|-------------|
| **NASO-MVS-001** | Mutable alias detected | Two `inout` parameters point to overlapping memory regions |
| **NASO-MVS-002** | Mutable access after function exit | Reference to mutated value used after function return |
| **NASO-MVS-003** | Concurrent mutable access in parallel context | Detected race condition in polyhedral-transformed loop |
| **NASO-MVS-004** | Immutable borrow after mutable borrow | Violates Rust-style borrowing rules (though Naso prevents this by default) |
| **NASO-MVS-005** | Move out of borrowed content | Attempted to move from an `inout` parameter |

### Uncomputation Errors (NASO-UNC-*)
| Code | Message | Description |
|------|---------|-------------|
| **NASO-UNC-001** | Entangled qubit dropped without measurement/uncomputation | Linear qubit involved in entanglement went out of scope |
| **NASO-UNC-002** | Qubit allocated but not consumed exactly once | `[1]` qubit count ≠ 1 at scope boundaries |
| **NASO-UNC-003** | Uncompute block contains irreversible operation (measurement) | Measurement inside `uncompute` breaks adjoint property |
| **NASO-UNC-004** | Ancilla not returned to \|0⟩ state | Final state verification failed for temporary qubits |
| **NASO-UNC-005** | Quantum control flow dependency on linear state | Branching on linear qubit measurement without proper uncomputation |

### Polyhedral Errors (NASO-POLY-*)
| Code | Message | Description |
|------|---------|-------------|
| **NASO-POLY-001** | Loop carry dependency prevents affine transformation | True data dependence found in loop nest |
| **NASO-POLY-002** | Non-affine array access detected | Subscript cannot be expressed as affine function |
| **NASO-POLY-003** | Dependence distance exceeds tile size | Vectorization blocked by loop-carried dependence |
| **NASO-POLY-004** | Imperfectly nested loops prevent transformation | Control flow breaks loop nest structure |
| **NASO-POLY-005** | Quantity scaling violation in transformed loop | Bounded quantity `[N]` doesn't scale correctly under affine map |

### QTT Errors (NASO-QTT-*)
| Code | Message | Description |
|------|---------|-------------|
| **NASO-QTT-001** | Erased `[0]` variable used in runtime context | Attempted to use proof value after codegen |
| **NASO-QTT-002** | Bounded quantity `[N]` exceeded static limit | Loop index proved to be ≥ N |
| **NASO-QTT-003** | Quantity arithmetic overflow in type checker | `q1 + q2` resulted in undefined quantity |
| **NASO-QTT-004** | Invalid quantity in function signature | Used `[2]` or other non-standard quantity |
| **NASO-QTT-005** | Polymorphic quantity constraint unsatisfiable | No solution for quantity variables in generics |

### SMT Solver Errors (NASO-SMT-*)
| Code | Message | Description |
|------|---------|-------------|
| **NASO-SMT-001** | Solver timeout exceeded | Verification took too long (consider simplifying) |
| **NASO-SMT-002** | Solver returned unknown | Z3 could not prove or disprove the property |
| **NASO-SMT-003** | Theory combination incomplete | Mixed theories (e.g., arrays + bitvectors) not supported |
| **NASO-SMT-004** | Assertion contradicts context | User assertion is logically impossible given preconditions |
| **NASO-SMT-005** | Quantifier instantiation failed | Failed to find suitable instances for quantified variables |

## Proof Sketch: Static Safety Soundness Theorem

**Theorem:** If a Naso program passes `naso-verify` without errors, then:
1. No memory leaks occur (all allocated resources are freed)
2. No race conditions exist on mutable data
3. Quantum states remain pure (no accidental measurement/collapse)
4. All linear resources are consumed exactly once
5. All erased quantities ([0]) are eliminated during code generation
6. All array accesses are within bounds
7. All polyhedral transformations preserve semantics

**Proof Sketch:**

### Lemma 1: Memory Safety (MVS & QTT)
*Allocation/deallocation balance and linear usage imply no leaks or dangling pointers.*

**Proof:**
- By QTT Lemma L1: Every `[1]`-typed allocation (`qalloc`, `alloc`) introduces exactly one linear token
- By MVS Lemma M1: Linear tokens must be consumed exactly once (no drops, no duplication)
- By QTT Lemma L2: `[0]` tokens are erased and consume no resources
- Therefore: ∀ allocation ∃ exactly one consumption/deallocation → No leaks

### Lemma 2: Race Freedom (MVS)
*Exclusive mutable access implies no data races.*

**Proof:**
- By MVS Lemma M2: At any program point, ∃ at most one active `inout` reference to a memory location
- By QTT Lemma L3: `[1]` and `[0]` quantities prevent aliasing by construction
- Therefore: Concurrent read/write or write/write impossible → Race-free

### Lemma 3: Quantum State Purity (Uncomputation)
*All temporary quantum states are returned to |0⟩ or measured.*

**Proof:**
- By Uncomputation Lemma U1: Every `uncompute` block implements \( U^\dagger \) for its forward computation U
- By QTT Lemma L4: `[1]` qubits must be consumed exactly once → No dropped entanglement
- By U2: Measurements only permitted on `[*]` or `[0]` qubits (ancilla must be `[*]` for reuse)
- Therefore: All temporary qubits either |0⟩ (reusable) or measured (collapsed to classical) → Pure state evolution

### Lemma 4: Polyhedral Semantic Preservation
*Affine loop transformations preserve data dependencies and termination.*

**Proof:**
- By Polyhedral Lemma P1: Dependence vectors \( \vec{d} \) satisfy \( \vec{d} \cdot \vec{\lambda} \geq 0 \) for legal schedule \( \vec{\lambda} \)
- By P2: Tiling, skewing, and interchange preserve \( \vec{d} \cdot \vec{\lambda} \geq 0 \) when \( \vec{\lambda} \) is updated accordingly
- By P3: Vectorization/parallelism only applied when dependence distance ≥ vector width/zero
- Therefore: Transformed loop nest computes same mathematical function as original

### Lemma 5: QTT Erasure Correctness
*All `[0]`-typed values are replaced with `void` during code generation.*

**Proof:**
- By QTT Lemma L5: `[0]` types inhabit only proof irrelevance types (all values observationally equal)
- By L6: No term of `[0]` type can affect control flow or memory (erasure is semantics-preserving)
- Therefore: Replace `[0]` values with `()`/`void` → Behaviorally equivalent code

### Lemma 6: Bounded Quantity Soundness
*`[N]` quantities never exceed their static bound N during execution.*

**Proof:**
- By QTT Lemma L7: Loop bound `[N]` translates to loop condition `i < N` in generated code
- By L8: Induction variable increments tracked in SMT → Proven `0 ≤ i < N` at all loop iterations
- By L9: Array accesses guarded by same bound → No out-of-bounds access
- Therefore: All `[N]` bounded accesses safe

### Theorem Proof (Compositionality)
The type system is syntax-directed and compositional:
- Each language construct preserves Lemmas 1-6 locally
- Sequential composition: Safety ∧ Safety → Safety
- Conditional composition: Safety preserved in both branches (path-sensitive)
- Loop composition: Invariant preservation → Safety after fixed-point
- Function composition: Pre/post conditions compose → Safety preserved

**QED.**

## Verification Workflow

### Basic Usage

```bash
# Verify entire project
naso verify

# Verify specific module
naso verify src/my_module.naso

# Verify specific function
naso verify src/my_module.naso::my_function

# Show SMT-LIB2 output for debugging
naso verify --emit-smt src/my_module.naso::my_function
```

### Output Format

```
[INFO] Verifying src/my_module.naso
[PASS] Linear type checking: 24/24 checks passed
[PASS] MVS alias analysis: 8/8 checks passed
[PASS] Uncomputation validity: 5/5 checks passed
[PASS] Polyhedral dependence: 12/12 checks passed
[PASS] QTT resource bounds: 16/16 checks passed
[PASS] Array bounds: 20/20 checks passed
[SUCCESS] All verification checks passed (0.123s)
```

### Failed Verification Example

```
[INFO] Verifying src/buffer.naso
[FAIL] Linear type checking: NASO-LIN-002 at buffer.naso:45:12
        Linear variable `buf` used more than once
        --> buffer.naso:45:12
         |
        45 |     let len = buf.len();   // First use
         |     let slice = &buf[0..5];  // Second use (ERROR)
         |     buf.clear();             // Third use
         |     ^^^^^^^^^^^^^^^^^^^^^^
        Help: Consider using `let slice = buf.slice(0..5);` or reordering operations
[ERROR] Verification failed
```

### Incremental Verification

```toml
# Cargo.toml
[package]
verify = { 
    incremental = true,
    cache-size = "100MB",
    timeout-seconds = 30
}
```

```bash
# Only verify changed functions since last build
naso verify --incremental
```

## Advanced Features

### Function Contracts

```naso
#[requires(x > 0 && y > 0)]
#[ensures(result == old(x) + old(y))]
fn add_positive(x: i32, y: i32) -> i32 {
    x + y
}
```

### Loop Invariants

```naso
fn sum(arr: [*] [N] i32) -> i32 {
    let mut acc = 0;
    #[invariant(0 <= i && i <= N && acc == sum(arr[0..i]))]
    for i in 0..N {
        acc += arr[i];
    }
    acc
}
```

### Assertions with Proof Hints

```naso
fn sqrt_newton(mut x: f64, guess: f64) -> f64 {
    #[assert(x >= 0.0, hint = "non-negative input")]
    #[assert(guess > 0.0, hint = "positive initial guess")]
    for _ in 0..10 {
        #[assert(x >= 0.0, hint = "non-negative preserved by iteration")]
        x = 0.5 * (x + guess / x);
    }
    x
}
```

### Verification Attributes

| Attribute | Description |
|-----------|-------------|
| `#[requires(condition)]` | Precondition that must hold on function entry |
| `#[ensures(condition)]` | Postcondition that must hold on function exit (uses `old()` for prestate values) |
| `#[invariant(condition)]` | Loop invariant that must hold at start and end of each iteration |
| `#[assert(condition, hint = "message")]` | Mid-function assertion with optional hint for SMT solver |
| `#[verify]` | Enable verification for this function (default for `pub` functions) |
| `#[no_verify]` | Disable verification for this function |

## Performance Considerations

### Verification Time Complexity

| Property | Complexity | Notes |
|----------|------------|-------|
| Linear type checking | O(n) | Single pass through AST |
| MVS alias analysis | O(n²) | Worst-case for pointer aliasing |
| QTT resource checking | O(n log n) | Constraint solving with simplex |
| Polyhedral dependence | O(n³) | Dependence analysis (Dominator-based) |
| SMT solving | NP-hard | Depends on theory and instance size |

### Optimization Tips

1. **Keep functions small**: Verification scales with function size
2. **Use concrete types**: Avoid generics when possible for faster monomorphization
3. **Provide loop invariers**: Helps SMT solver converge faster
4. **Use `#[assert]` hints**: Guide quantifier instantiation
5. **Enable incremental verification**: Cache results between builds
6. **Consider `--no-verify` for debug builds**: Faster iteration during development

## Integration with Build System

### Cargo.toml Configuration

```toml
[package]
verify = { 
    default = true,
    level = "moderate",   // Options: none, minimal, moderate, aggressive
    timeout-seconds = 60,
    incremental = true,
    emit-smt = false      // Set to true for debugging
}

[dependencies]
naso-std = { version = "0.1", features = ["smt"] }
```

### Custom Verification Levels

- `none`: Skip all verification (fastest, least safe)
- `minimal`: Only check `[0]` erasure and obvious linear drops
- `moderate`: Default - all guarantees except full polyhedral dependence
- `aggressive`: All guarantees + loop invariants + function contracts

### Continuous Integration Example

```yaml
# .github/workflows/verify.yml
name: Verification

on: [push, pull_request]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo naso verify --release
    - run: cargo test --release
```

---