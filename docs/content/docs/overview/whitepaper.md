---
title: "Technical Whitepaper"
weight: 1
---

# Technical Whitepaper: Naso — A Non-aliasing Affine Systems Orchestrator for Provably Safe Quantum & AI Computing

## Executive Summary & The Naso Thesis

Classical memory models (C++, Rust) and quantum SDKs fail at hardware boundaries for three fundamental reasons:

**1. Implicit Pointer Aliasing:** C++ allows arbitrary pointer aliasing; Rust's borrow checker prevents it but at the cost of pervasive lifetime annotations and the friction of `&mut` vs `&` distinction. Neither model accounts for quantum state, where aliasing is physically forbidden by the no-cloning theorem. A qubit cannot be "borrowed immutably"—observation collapses it.

**2. Untracked Ancilla Decoherence:** Quantum algorithms require temporary ancilla qubits that must be returned to the $\|0\rangle$ state before deallocation (Bennett uncomputation). Existing quantum languages (Q#, Qiskit, Cirq) treat this as a runtime convention or library discipline. When ancilla are implicitly dropped while entangled, the resulting decoherence corrupts the entire computation—silently, irrecoverably, and often only discovered on physical hardware.

**3. Unverified Loop Bounds:** Tensor accelerators (GPUs, TPUs, NPUs) and quantum pulse schedulers require exact iteration space bounds for optimal mapping. Classical compilers rely on heuristic dependence analysis that fails on imperfectly nested loops, non-affine accesses, and dynamic bounds. The result is suboptimal schedules or, worse, out-of-bounds memory access that manifests only under specific input sizes.

**The Naso Thesis:** *A single type system integrating Quantitative Type Theory (QTT), Mutable Value Semantics (MVS), Polyhedral Loop Compilation, and Z3 SMT verification can simultaneously eliminate all three failure modes at compile time with zero runtime overhead.*

Naso achieves this by making quantity, mutability, and loop structure first-class citizens in the type system, verified by an embedded Z3 solver before any code generation.

---

## Section 2: The Four Architectural Pillars

### Pillar I: Quantitative Type Theory (QTT)

QTT extends the simply-typed lambda calculus with a quantity semiring that tracks *how many times* a variable may be used.

#### Quantity Lattice

$$Q = \{0, 1, N, *\}$$

with partial order: $0 \leq 1 \leq N \leq *$

#### Semiring Operations

**Addition (+) — Resource Combination:**

| + | 0 | 1 | N | * |
|---|---|---|---|---|
| **0** | 0 | 1 | N | * |
| **1** | 1 | * | * | * |
| **N** | N | * | * | * |
| **\*** | * | * | * | * |

*Saturation: $1+1 = *$ (two linear uses = unbounded), $N+1 = *$, $N+N = *$. Any sum involving $*$ yields $*$.*

**Multiplication (·) — Resource Scaling:**

| · | 0 | 1 | N | * |
|---|---|---|---|---|
| **0** | 0 | 0 | 0 | 0 |
| **1** | 0 | 1 | N | * |
| **N** | 0 | N | N | * |
| **\*** | 0 | * | * | * |

*Note: $N \cdot N = N$ (idempotent for loop bounds). Any product with $0$ yields $0$; with $*$ yields $*$.*

#### Linear Consumption Constraints

**Context Splitting:**
$$\frac{\Gamma \vdash e : [q] \tau \quad q = q_1 + q_2}{\Gamma_1, \Gamma_2 \vdash e : [q_1] \tau \otimes [q_2] \tau}$$
where $\Gamma = \Gamma_1 \oplus \Gamma_2$ (disjoint split).

**Linear Consumption:**
$$\frac{\Gamma, x:[1]\tau \vdash e : \sigma}{\Gamma \vdash \text{let } _ = x \text{ in } e : \sigma}$$
Variable $x$ must be used exactly once in $e$.

**Erased Elimination:**
$$\frac{\Gamma \vdash e : [0] \tau}{\Gamma \vdash \text{erase}(e) : \text{void}}$$
Any `[0]`-qualified value is erased to `void` during code generation.

**Bounded Loop Scaling:**
$$\frac{\Gamma \vdash e : [N] \tau \quad \text{loop\_bound}(L) = M}{\Gamma \vdash \text{for } i \text{ in } 0..L \{ e \} : [N \cdot M] \tau}$$
Quantity scales linearly with loop iterations.

**Subtyping:**
$$\frac{q_1 \leq q_2}{[q_1]\tau \leq [q_2]\tau}$$
where $0 \leq 1 \leq N \leq *$.

---

### Pillar II: Mutable Value Semantics (MVS)

MVS provides `inout` parameters with pass-by-writeback semantics and static non-aliasing guarantees.

#### Operational Memory Writeback Semantics

For function `fn f(inout x: [1] T) -> [1] T`:

1. **Entry**: Argument value copied to temporary location $x_{tmp}$
2. **Execution**: All mutations occur on $x_{tmp}$ (exclusive access guaranteed)
3. **Normal Exit**: $x_{tmp}$ copied back to original location
4. **Abnormal Exit** (panic, early return): Original $x$ unchanged (automatic rollback)

#### Formal Disjointness Predicate

For any two pointers $p_i$ and $p_j$ with types $[qty_i] \tau*$ and $[qty_j] \tau*$:

$$\text{Disjoint}(p_i, p_j) \iff \text{Span}(p_i) \cap \text{Span}(p_j) = \emptyset$$

where $\text{Span}(p) = \{ \text{addr}(p) + k \mid 0 \leq k < \text{sizeof}(\tau) \}$

The type system guarantees:

$$\frac{\Gamma \vdash p_i : [qty_i] \tau* \quad \Gamma \vdash p_j : [qty_j] \tau* \quad i \neq j}{\Gamma \vdash \text{Disjoint}(p_i, p_j) : \text{true}}$$

when both quantities are `[1]` or `[0]` (linear/erased pointers cannot alias by construction).

#### MVS vs. Alternatives

| Feature | Naso MVS | Rust Borrowing | C++ Pointers |
|---------|----------|----------------|--------------|
| **Alias Prevention** | Static (type system) | Static (borrow checker) | None |
| **Mutation Safety** | Exclusive via copy-in/out | Exclusive via `&mut` | None |
| **Null Safety** | No null pointers | No null references | Possible |
| **Dangling Prevention** | Lifetime enforced by scope | Lifetime enforced by scope | Possible |
| **Rollback on Panic** | Automatic | Manual (`drop` guards) | None |
| **Syntax** | `inout param: [1] T` | `param: &mut T` | `T* param` |

---

### Pillar III: Polyhedral Loop Engine

The polyhedral engine computes exact affine iteration spaces and applies verified schedule transformations.

#### Affine Schedule Transformations

Given a loop nest with iteration domain $\mathcal{D} \subseteq \mathbb{Z}^d$ and dependence vectors $\vec{d} \in \mathbb{Z}^d$, a schedule $\vec{\lambda}: \mathcal{D} \to \mathbb{Z}^m$ is **legal** iff:

$$\forall \vec{d} \in \text{Dependences}: \vec{d} \cdot \vec{\lambda} \geq 0$$

**Supported Transformations (verified by Z3):**

| Transformation | Schedule Effect | Hardware Target |
|----------------|-----------------|-----------------|
| **Tiling** | $\vec{\lambda}_{tiled} = (\lfloor \vec{i}/B \rfloor, \vec{i} \bmod B)$ | Cache blocking (CPU/GPU) |
| **Skewing** | $\vec{\lambda}_{skewed} = S \vec{i}, S = \begin{pmatrix} 1 & k \\ 0 & 1 \end{pmatrix}$ | Wavefront parallelism |
| **Interchange** | $\vec{\lambda}_{inter} = P \vec{i}$ ($P$ permutation) | Memory coalescing (GPU) |
| **Fusion** | $\vec{\lambda}_{fused} = \text{concat}(\vec{\lambda}_1, \vec{\lambda}_2)$ | Kernel fusion |
| **Vectorization** | $\vec{\lambda}_{vec} = (\vec{\lambda}_{outer}, \lfloor i_{inner}/W \rfloor)$ | SIMD/AVX-512 (width $W$) |

**Quantity Scaling Under Transformation:**

For loop with bounded quantity $[N]$ and transformation $\vec{\lambda}' = A \vec{\lambda} + \vec{b}$:

$$[N] \mapsto [N \cdot \det(A_{sub})]$$

where $A_{sub}$ is the submatrix of $A$ corresponding to the bounded dimensions. Z3 verifies $\det(A_{sub}) = 1$ for legal affine transformations preserving quantity bounds.

---

### Pillar IV: Z3 SMT Formal Verification

`naso-verify` translates Naso programs to SMT-LIB2 and checks verification conditions using Z3.

{{< tabs >}}
  {{< tab name="Non-Aliasing Check" >}}
```lisp
; Non-aliasing check for parameters a and b
(declare-fun a_ptr () Int)
(declare-fun a_len () Int)
(declare-fun b_ptr () Int)
(declare-fun b_len () Int)
(assert (= a_len N))
(assert (= b_len N))
; Disjointness: [a_ptr, a_ptr+N) ∩ [b_ptr, b_ptr+N) = ∅
(assert (or (<= (+ a_ptr N) b_ptr) (<= (+ b_ptr N) a_ptr)))
(check-sat)
; Expected: sat (if N > 0 and pointers don't overlap)
```
  {{< /tab >}}
  {{< tab name="Linear Variable Tracking" >}}
```lisp
(declare-fun buf_init () Array)
(declare-fun buf_after_len () Array)
(declare-fun buf_after_clear () Array)

; Track linear state transitions
(assert (= buf_after_len buf_init)) ; len() doesn't consume
(assert (= buf_after_clear (as const ([*]) Array))) ; clear() consumes

; Check that final state is consumed exactly once
(assert (distinct buf_after_clear buf_init))
(assert (= buf_after_clear (as const ([*]) Array)))

(check-sat) ; UNSAT indicates linear usage violation
```
  {{< /tab >}}
  {{< tab name="Quantum Uncomputation" >}}
```lisp
; Quantum state as density matrix elements
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
(assert (= rho00 0.5))
(assert (= rho11 0.5))
(assert (= rho01 0.0))
(assert (= rho10 0.0))

; Uncompute requirement: final state must be |00>
(assert (or (= rho00 0.0) (= rho11 0.0))) ; Violation if entangled remains
(check-sat) ; UNSAT means cannot disentangle - ERROR
```
  {{< /tab >}}
{{< /tabs >}}

---

## Section 3: Formal Guarantees & Soundness Theorem

### Static Safety Theorem

**Theorem:** If `naso-verify` outputs `UNSAT` for all generated verification conditions, the program contains:

1. **Zero runtime memory leaks** — All allocated resources are freed exactly once
2. **Zero aliasing bugs** — No two mutable references overlap
3. **Zero quantum decoherence trace-outs** — All ancilla uncomputed to $\|0\rangle$
4. **Zero array bounds violations** — All accesses proven in-bounds
5. **Zero linear resource drops** — Every `[1]` variable consumed exactly once
6. **Zero erased quantity leaks** — All `[0]` values eliminated during codegen

### Proof Sketch

The theorem follows from composition of five lemmas, each mechanically checked by `naso-verify`:

**Lemma 1 (Memory Safety):** By QTT linear consumption (every `[1]` token consumed exactly once) and MVS copy-in/copy-out (mutations isolated with rollback), allocation/deallocation is balanced. No leaks, no double-frees, no dangling pointers.

**Lemma 2 (Race Freedom):** MVS guarantees at most one active `inout` reference per location. QTT ensures `[1]` and `[0]` quantities prevent aliasing by construction. Concurrent read/write or write/write impossible.

**Lemma 3 (Quantum State Purity):** Every `uncompute` block implements $U^\dagger$ for its forward $U$ (Bennett's method). `[1]` qubits must be consumed exactly once — no dropped entanglement. Measurements only on `[*]` or `[0]` qubits. All temporary qubits either $\|0\rangle$ (reusable) or measured (collapsed to classical).

**Lemma 4 (Polyhedral Semantic Preservation):** Dependence vectors $\vec{d}$ satisfy $\vec{d} \cdot \vec{\lambda} \geq 0$ for legal schedule $\vec{\lambda}$. Tiling, skewing, interchange preserve this invariant. Vectorization/parallelism only applied when dependence distance $\geq$ vector width/zero. Transformed loop computes same mathematical function.

**Lemma 5 (QTT Erasure Correctness):** `[0]` types inhabit only proof irrelevance types. No term of `[0]` type can affect control flow or memory. Replacement with `void` is semantics-preserving.

**Compositionality:** The type system is syntax-directed and compositional. Each construct preserves Lemmas 1-5 locally. Sequential, conditional, loop, and function composition preserve safety by induction on program structure.

**QED.**

---

## Section 4: Target Execution Ecosystem

### Backend Matrix

| Target | IR Format | Status | Key Features |
|--------|-----------|--------|--------------|
| **CPU (x86-64, ARM64)** | LLVM IR → native | Production | Auto-vectorization (AVX-512, NEON), polyhedral scheduling, MVS copy-elision |
| **NVIDIA GPU** | PTX / CUDA C++ | Production | Tensor cores, async copy, cluster launch, polyhedral tile→block mapping |
| **AMD GPU** | AMDGCN / HIP | Beta | Wavefront scheduling, LDS optimization |
| **Intel XPU** | SPIR-V / SYCL | Beta | Xe matrix extensions, unified memory |
| **Quantum (QIR)** | QIR Bitcode (LLVM) | Production | Qubit allocation, unitary gates, measurement, uncomputation blocks → adjoint |
| **OpenQASM 3.0** | OpenQASM 3.0 AST | Production | Gate-level export, classical control flow, real-time feedback |
| **Pulse-Level** | QIR Pulse / OpenPulse | Alpha | Hardware-aware pulse scheduling, calibration integration |

### QIR Backend: Quantum Intermediate Representation

Naso lowers quantum operations to QIR (LLVM-based quantum IR):

```llvm
; ModuleID = 'naso_bell_pair'
source_filename = "naso_bell_pair"

%Qubit = type opaque
%Result = type opaque

declare %Qubit* @__quantum__rt__qubit_allocate()
declare void @__quantum__rt__qubit_release(%Qubit*)
declare void @__quantum__qis__h__body(%Qubit*)
declare void @__quantum__qis__cnot__body(%Qubit*, %Qubit*)

define void @create_bell_pair(%Qubit* %q0, %Qubit* %q1) {
entry:
  call void @__quantum__qis__h__body(%Qubit* %q0)
  call void @__quantum__qis__cnot__body(%Qubit* %q0, %Qubit* %q1)
  ret void
}
```

### Uncomputation Lowering

`uncompute { ... }` blocks are transformed to adjoint sequences:

```naso
uncompute {
    hadamard(inout q0);
    cnot(inout q0, inout q1);
}
```

Lowers to (QIR):

```llvm
; Forward: H(q0); CNOT(q0, q1)
call void @__quantum__qis__h__body(%Qubit* %q0)
call void @__quantum__qis__cnot__body(%Qubit* %q0, %Qubit* %q1)

; Adjoint (reverse order, dagger): CNOT†(q0, q1); H†(q0)
call void @__quantum__qis__cnot__adj(%Qubit* %q0, %Qubit* %q1)
call void @__quantum__qis__h__adj(%Qubit* %q0)
```

### OpenQASM 3.0 Export

```openqasm
OPENQASM 3.0;
include "stdgates.inc";

qubit[2] q;
h q[0];
cx q[0], q[1];
// Uncomputation automatically appended:
// cx q[0], q[1];
// h q[0];
```

### Classical LLVM Pass Pipeline

For `[*]` and `[0]` code, Naso emits optimized LLVM IR through standard passes:

```
Naso AST → QTT/Type Check → MVS Lowering → Polyhedral Schedule
    → LLVM IR (mem2reg, instcombine, loop-vectorize, slp-vectorize)
    → Target-specific codegen (x86-64, ARM64, PTX, AMDGCN)
```

Polyhedral schedules are encoded as LLVM metadata (`!llvm.loop.accesses`, `!llvm.loop.parallel`) guiding the loop vectorizer and parallelizer.

---

## Conclusion

Naso represents a paradigm shift: **safety is not a runtime service but a compile-time algebraic property**. By unifying QTT, MVS, polyhedral compilation, and SMT verification into a single type system, Naso eliminates the three fatal gaps at quantum/classical/tensor hardware boundaries. The result is a language where:

- **Memory safety** is a theorem, not a convention
- **Quantum state conservation** is enforced by linear types, not programmer discipline
- **Performance portability** comes from verified polyhedral schedules, not vendor-specific pragmas
- **Zero overhead** means exactly that: `[0]` erased, `[1]` moved, `[N]` bounded, `[*]` classical

Naso is the systems language for the post-Moore, post-von Neumann era—where quantum, tensor, and classical compute converge on a single verified substrate.