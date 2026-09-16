---
title: "Naso — Formal-Verification-Driven Systems Language"
layout: hextra-home
---

<div class="hx-mt-12 hx-text-center">
  <h1 class="hx-text-5xl hx-font-bold hx-tracking-tight hx-text-slate-100">
    Formal Verification Meets Quantum & AI Engineering
  </h1>
  <p class="hx-mt-6 hx-text-xl hx-text-slate-400 hx-max-w-3xl hx-mx-auto">
    Naso is a systems programming language featuring Quantitative Type Theory (QTT), Polyhedral Loop Compilation, and an embedded Z3 SMT prover for provably safe hardware execution.
  </p>
  <div class="hx-mt-8 hx-flex hx-justify-center hx-gap-4">
    <a href="/docs/qtt/" class="hx-bg-cyan-500 hover:hx-bg-cyan-600 hx-text-slate-950 hx-font-semibold hx-px-6 hx-py-3 hx-rounded-lg hx-transition">
      Get Started
    </a>
    <a href="/docs/spec/" class="hx-border hx-border-slate-700 hover:hx-border-slate-500 hx-text-slate-200 hx-font-semibold hx-px-6 hx-py-3 hx-rounded-lg hx-transition">
      Language Spec
    </a>
  </div>
</div>

<div class="hx-mt-20">
{{< hextra/feature-grid >}}
  {{< hextra/feature-card
    title="Quantitative Type Theory (QTT)"
    subtitle="Track compile-time resource bounds using [0] erased proofs, [1] linear resources, and [*] classical variables directly in the type system."
    style="background: linear-gradient(180deg, rgba(17,24,39,0.8) 0%, rgba(11,15,25,1) 100%); border: 1px solid #1e293b;"
  >}}
  {{< hextra/feature-card
    title="Polyhedral Loop Engine"
    subtitle="Affine loop transformations automatically map matrix contractions to optimal schedules while enforcing Bennett quantum uncomputation invariants."
    style="background: linear-gradient(180deg, rgba(17,24,39,0.8) 0%, rgba(11,15,25,1) 100%); border: 1px solid #1e293b;"
  >}}
  {{< hextra/feature-card
    title="Z3 Formal Prover (naso-verify)"
    subtitle="Integrated SMT verification checks non-aliasing, linear state leaks, and quantum state uncomputation before code generation."
    style="background: linear-gradient(180deg, rgba(17,24,39,0.8) 0%, rgba(11,15,25,1) 100%); border: 1px solid #1e293b;"
  >}}
{{< /hextra/feature-grid >}}
</div>