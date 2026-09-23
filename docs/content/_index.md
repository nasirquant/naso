---
title: "Naso — Non-aliasing Affine Systems Orchestrator"
layout: hextra-home
---

{{< hextra/hero-badge >}}
  <div class="hx:w-2 hx:h-2 hx:rounded-full hx:bg-primary-400"></div>
  <span>Quantitative Type Theory · Mutable Value Semantics · Polyhedral Loops · Z3 SMT Prover</span>
{{< /hextra/hero-badge >}}

<div class="hx:mt-6 hx:mb-6">
{{< hextra/hero-headline >}}
  Naso: Non-aliasing Affine Systems Orchestrator
{{< /hextra/hero-headline >}}
</div>

<div class="hx:mb-12">
{{< hextra/hero-subtitle >}}
  A provably safe systems language unifying QTT, MVS, polyhedral compilation, and Z3 verification for quantum & heterogeneous AI hardware.
{{< /hextra/hero-subtitle >}}
</div>

<div class="hx:mb-6 hx:grid hx:gap-4">
{{< hextra/hero-button text="Read the Technical Whitepaper" link="/docs/overview/whitepaper/" >}}
{{< hextra/hero-button text="Try Playground" link="https://play.nasolang.org/" variant="secondary" >}}
</div>

{{< callout type="info" emoji="🎯" >}}
**Executive Summary:** Closing the quantum/tensor safety gap. Naso unifies QTT, MVS, polyhedral compilation, and Z3 verification into a single zero-overhead language where memory leaks, aliasing bugs, and quantum decoherence are compile-time type errors.
{{< /callout >}}

<div class="hx:mt-6 hx:mb-6">
{{< hextra/hero-section heading="h2" >}}Four Core Pillars{{< /hextra/hero-section >}}
</div>

{{< cards >}}
  {{< card link="/docs/overview/whitepaper/#pillar-i-quantitative-type-theory-qtt" title="🧮 Quantitative Type Theory (QTT)" subtitle="Tracks resource quantities ([0], [1], [N], [*]) directly in the type system. Prevents quantum state leaks and double-use at compile time with zero runtime overhead." >}}
  {{< card link="/docs/overview/whitepaper/#pillar-ii-mutable-value-semantics-mvs" title="🔁 Mutable Value Semantics (MVS)" subtitle="Pass-by-writeback with strict non-aliasing guarantees. `inout` provides exclusive mutable access with copy-in/copy-out semantics—zero pointer borrowing overhead." >}}
  {{< card link="/docs/overview/whitepaper/#pillar-iii-polyhedral-loop-engine" title="🔲 Polyhedral Loop Engine" subtitle="Static iteration space bounds via affine schedules for optimal quantum/tensor kernel scheduling. Tiling, skewing, interchange, fusion, vectorization verified by Z3 before hardware lowering." >}}
  {{< card link="/docs/overview/whitepaper/#pillar-iv-z3-smt-formal-verification" title="🛡️ Z3 SMT Formal Verification" subtitle="Embedded naso-verify generates SMT-LIB2 invariants to eliminate runtime panics, buffer overflows, race conditions, and quantum decoherence trace-outs at compile time." >}}
{{< /cards >}}

<div class="hx:mt-6 hx:mb-6">
{{< hextra/hero-section heading="h2" >}}Code / Syntax Preview{{< /hextra/hero-section >}}
</div>

<div class="hx:w-full hx:min-w-0 hextra-max-content-width hx:px-3 hx:pt-4 hx:md:px-12">
{{< tabs >}}
  {{< tab name="Quantum Circuit" >}}
```naso
fn create_bell_pair(inout q0: [1] Qubit, inout q1: [1] Qubit) -> [0] Proof {
    hadamard(inout q0);
    cnot(inout q0, inout q1);
    // q0, q1 now form a Bell pair |00⟩ + |11⟩
    // Automatic uncomputation on scope exit
}
```
  {{< /tab >}}
  {{< tab name="Polyhedral Tensor Kernel" >}}
```naso
fn matmul(A: [N][M] f64, B: [M][P] f64) -> [N][P] f64 {
    let C = [[0.0; P]; N];
    for i in 0..N {        // [*] outer loop
        for j in 0..P {    // [*] outer loop
            for k in 0..M { // [M] bounded by polyhedral domain
                C[i][j] += A[i][k] * B[k][j];
            }
        }
    }
    C
}
```
  {{< /tab >}}
  {{< tab name="Z3 Verification" >}}
```naso
fn safe_get(arr: [*] [N] f64, idx: usize) -> f64 {
    let mut solver = new_solver();
    let idx_e = expr_from(idx);
    let n_e = expr_from(N);
    
    // Prove 0 <= idx < N at compile time
    assert(&mut solver, idx_e >= expr_from(0));
    assert(&mut solver, idx_e < n_e);
    
    match check(&solver) {
        CheckResult::Sat => arr[idx],           // Bounds proven — no runtime check
        CheckResult::Unsat => compile_error!("Index out of bounds"),
        CheckResult::Unknown => if idx < N { arr[idx] } else { panic!() }
    }
}
```
  {{< /tab >}}
{{< /tabs >}}
</div>

<div class="hx:mt-6 hx:mb-6">
{{< hextra/hero-section heading="h2" >}}Philosophy & FDR Teaser{{< /hextra/hero-section >}}
</div>

{{< callout type="warning" emoji="🚫" >}}
**Frequently Denied Requests (FDR) — Teaser**

- **FDR-001**: Request for `any` or `void*` untyped pointers → *Status: Permanently Denied*. Z3 solver experienced existential dread and refused to generate SMT-LIB2 output.
- **FDR-002**: Request to drop entangled ancilla qubits implicitly → *Status: Permanently Denied*. Schrödinger's cat called our legal team.
- **FDR-003**: Request for background garbage collector thread → *Status: Permanently Denied*. Non-deterministic execution pauses violate the laws of physics and engineering decency.
- **FDR-004**: Request to suppress compiler errors on linear variable reuse → *Status: Permanently Denied*. The quantum no-cloning theorem is not a toggleable compiler flag.

[Read the full Philosophy & Zen →](/docs/philosophy/)
{{< /callout >}}

{{< cards >}}
  {{< card link="/docs/overview/whitepaper/" title="Technical Whitepaper" subtitle="Deep dive into the four architectural pillars, formal guarantees, and target execution ecosystem." >}}
  {{< card link="/docs/spec/" title="Language Specification" subtitle="Complete formal grammar, type rules, operational semantics, and verification engine details." >}}
  {{< card link="/docs/philosophy/" title="Philosophy & Zen" subtitle="The four axioms, ten aphorisms, explicit non-goals, and the full FDR list." >}}
  {{< card link="/docs/stdlib/" title="Standard Library" subtitle="API reference for `std::quantum`, `std::poly`, `std::smt`, `std::mem`, `std::math`." >}}
  {{< card link="/docs/verification/" title="Formal Verification" subtitle="naso-verify SMT engine, error code matrix, soundness proofs, and workflow." >}}
  {{< card link="/docs/qtt/" title="Quantitative Type Theory" subtitle="Deep dive into QTT quantities, semiring, typing judgments, and advanced examples." >}}
{{< /cards >}}