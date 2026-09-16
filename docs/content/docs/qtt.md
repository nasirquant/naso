---
title: "Quantitative Type Theory (QTT)"
weight: 10
---

Naso integrates **Quantitative Type Theory (QTT)** into its core type-checker to govern memory safety, quantum state conservation, and zero-cost compile-time proofs without a garbage collector.

## Quantity Semantics

Quantities modulate variable declarations and restrict how many times a binding can be consumed:

| Quantity | Meaning | Runtime Effect | Linearity |
|----------|---------|---------------|-----------|
| `[0]` Erased | Compile-time proof or type witness | Fully eliminated during codegen | Irrelevant (affine) |
| `[1]` Linear | Single-use affine resource | Must be consumed or uncomputed exactly once | Linear |
| `[*]` Unbounded | Standard classical value | Unlimited reads, copies, implicit drops | Affine |
| `[N]` Bounded | Static quantity bounded by polyhedral domain | Bounded iteration count | Linear (scaled) |

---

## Formal Semiring Definition

Quantities form a semiring $Q = \{0, 1, N, *\}$ with operations:

### Addition (+) Table

| + | 0 | 1 | N | * |
|---|---|---|---|---|
| **0** | 0 | 1 | N | * |
| **1** | 1 | * | * | * |
| **N** | N | * | * | * |
| **\*** | * | * | * | * |

*Note: $1+1 = *$ (saturation), $N+1 = *$, $N+N = *$. Any sum involving $*$ yields $*$.*

### Multiplication (·) Table

| · | 0 | 1 | N | * |
|---|---|---|---|---|
| **0** | 0 | 0 | 0 | 0 |
| **1** | 0 | 1 | N | * |
| **N** | 0 | N | N | * |
| **\*** | 0 | * | * | * |

*Note: $N \cdot N = N$ (idempotent for loop bounds). Any product with $0$ yields $0$; with $*$ yields $*$.*

---

## Typing Judgments

### Context Splitting
$$\frac{\Gamma \vdash e : [q] \tau \quad q = q_1 + q_2}{\Gamma_1, \Gamma_2 \vdash e : [q_1] \tau \otimes [q_2] \tau}$$
where $\Gamma = \Gamma_1 \oplus \Gamma_2$ (disjoint split).

### Linear Consumption
$$\frac{\Gamma, x:[1]\tau \vdash e : \sigma}{\Gamma \vdash \text{let } _ = x \text{ in } e : \sigma}$$
Variable $x$ must be used exactly once in $e$.

### Erased Elimination
$$\frac{\Gamma \vdash e : [0] \tau}{\Gamma \vdash \text{erase}(e) : \text{void}}$$
Any `[0]`-qualified value is erased to `void` during code generation.

### Bounded Loop Scaling
$$\frac{\Gamma \vdash e : [N] \tau \quad \text{loop\_bound}(L) = M}{\Gamma \vdash \text{for } i \text{ in } 0..L \{ e \} : [N \cdot M] \tau}$$
Quantity scales linearly with loop iterations.

### Subtyping
$$\frac{q_1 \leq q_2}{[q_1]\tau \leq [q_2]\tau}$$
where $0 \leq 1 \leq N \leq *$.

---

## Code Lowering Comparison

The following example demonstrates a Bell pair circuit written in Naso, transformed into OpenQASM 3.0, and compiled down to QIR Bitcode:

{{< tabs >}}
  {{< tab name="Naso" >}}
```naso
fn create_bell_pair(inout q0: [1] Qubit, inout q1: [1] Qubit) -> [0] Proof {
    hadamard(inout q0);
    cnot(inout q0, inout q1);
}
```
  {{< /tab >}}
  {{< tab name="OpenQASM 3.0" >}}
OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
  {{< /tab >}}
  {{< tab name="QIR Bitcode" >}}
; ModuleID = 'naso_bell_pair'
source_filename = "naso_bell_pair"
define void @create_bell_pair(%Qubit* %q0, %Qubit* %q1) {
entry:
  %0 = call %Qubit* @hadamard(%Qubit* %q0)
  %1 = call %Qubit* @cnot(%Qubit* %q0, %Qubit* %q1)
  ret void
}
  {{< /tab >}}
  {{< tab name="LLVM IR (classical)" >}}
; Classical function with [0] proof erased
define void @create_bell_pair() {
entry:
  ret void
}
  {{< /tab >}}
{{< /tabs >}}

---

## Diagnostic Callouts

Naso provides rich compile-time diagnostics to help users write correct code.

{{< callout type="info" emoji="💡" >}}
**Info:** `[0]` proofs are erased at compile time — they incur **zero runtime overhead**.
{{< /callout >}}

{{< callout type="warning" emoji="⚠️" >}}
**Warning:** Mixing `[1]` and `[*]` in the same data structure requires explicit `box`/`unbox` — the type checker will guide you.
{{< /callout >}}

{{< callout type="error" emoji="🚫" >}}
**Error:** Linear variable `q` used twice — each `[1]` resource must be consumed exactly once. Use `uncompute` block or transfer ownership.
{{< /callout >}}

---

## Advanced Examples

### Erased Proof Carrying Code

```naso
// Proof that array is sorted — erased at runtime
fn verify_sorted(arr: [*] [N] i32) -> [0] Proof {
    for i in 0..N-2 {
        assert(arr[i] <= arr[i+1]); // Compile-time checked by naso-verify
    }
}

// Consumer receives proof that input is sorted
fn binary_search(arr: [*] [N] i32, key: i32, _proof: [0] Proof) -> Option<usize> {
    // Proof is erased, but guarantees arr is sorted at compile time
    let mut lo = 0;
    let mut hi = N;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if arr[mid] < key { lo = mid + 1; }
        else { hi = mid; }
    }
    if lo < N && arr[lo] == key { Some(lo) } else { None }
}
```

### Bounded Quantities in Polyhedral Loops

```naso
// N and M are polyhedral domain parameters
fn matmul(A: [N][M] f64, B: [M][P] f64) -> [N][P] f64 {
    let C = [[0.0; P]; N];
    
    // Outer loops: quantity [*] (unbounded classical)
    // Inner loop: quantity [M] (bounded by polyhedral domain)
    for i in 0..N {        // [*]
        for j in 0..P {    // [*]
            for k in 0..M { // [M] — scales with domain size
                C[i][j] += A[i][k] * B[k][j];
            }
        }
    }
    C // Returns [*] [N][P] f64
}
```

### Linear Resource Transfer

```naso
fn process_buffer(buf: [1] Vec<[N] u8>) -> [1] Vec<[N] u8> {
    // buf is moved into this function — caller can no longer use it
    let mut transformed = Vec::with_capacity(buf.len());
    
    for byte in buf {  // Consumes buf element by element
        transformed.push(byte ^ 0xFF); // Transform each byte
    }
    
    // buf is fully consumed here — OK
    transformed // Transfer ownership to caller
}

// Usage:
fn main() {
    let data = vec![1, 2, 3, 4]; // [*] Vec
    let linear_data = data.into_linear(); // Convert to [1] Vec (consumes data)
    let result = process_buffer(linear_data); // linear_data moved, result returned
    // linear_data is now invalid — compile error if used
    use(result);
}
```

---

## QTT in Function Signatures

### Complete Signature Grammar

```
fn name(
    [inout] param1: [qty1] type1,
    [inout] param2: [qty2] type2,
    ...
) -> [qty_ret] ret_type { body }
```

### Quantity Rules for Parameters

| Parameter Mode | Quantity | Ownership | Usage |
|----------------|----------|-----------|-------|
| `x: [1] T` | Linear | Moved in | Consumed exactly once |
| `inout x: [1] T` | Linear | Borrowed exclusively | Mutated, returned to caller |
| `x: [*] T` | Unbounded | Copied | Unlimited reads |
| `inout x: [*] T` | Unbounded | Borrowed mutably | Mutated, visible to caller |
| `x: [0] T` | Erased | Proof only | Erased, no runtime value |

### Return Quantity Rules

- `[0]`: Proof/guarantee — erased, no runtime value returned
- `[1]`: Linear resource — caller must consume exactly once
- `[*]`: Classical value — caller gets owned copy
- `[N]`: Bounded — size known at compile time

---

## Interaction with MVS and Uncomputation

### MVS + QTT: Mutable Value Semantics with Quantities

```naso
// inout with [1] = exclusive mutable access to linear resource
fn qubit_rotate(inout q: [1] Qubit, angle: f64) -> [0] Proof {
    // q is exclusively borrowed — no other references exist
    rx(inout q, angle); // Rotate around X-axis
    // q returned to caller in mutated state
}
```

### Uncomputation + QTT: Automatic Adjoint Generation

```naso
fn oracle(inout x: [1] [N] Qubit, inout ancilla: [1] Qubit) -> [0] Proof {
    // Forward computation
    for i in 0..N {
        cnot(inout x[i], inout ancilla); // Entangle
    }
    
    // uncompute block automatically generates adjoint
    uncompute {
        // This block is inverted and appended
        // Result: ancilla returned to |0⟩
    }
}
// After function: ancilla is |0⟩, x holds result
```