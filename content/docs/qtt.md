---
title: "Quantitative Type Theory (QTT)"
weight: 10
---

Naso integrates **Quantitative Type Theory (QTT)** into its core type-checker to govern memory safety, quantum state conservation, and zero-cost compile-time proofs without a garbage collector.

## Quantity Semantics

Quantities modulate variable declarations and restrict how many times a binding can be consumed:

* <span class="qtt-tag qtt-0">[0] Erased</span> — Compile-time proof or type witness. Fully eliminated during codegen ($[0] \to \text{void}$).
* <span class="qtt-tag qtt-1">[1] Linear</span> — Single-use affine resource. Must be consumed or uncomputed exactly once ($[1] \to \text{value}$).
* <span class="qtt-tag qtt-star">[*] Unbounded</span> — Standard classical value allowing unlimited reads, copies, and implicit drops.
* <span class="qtt-tag qtt-N">[N] Bounded</span> — Static quantity bounded by a polyhedral iteration domain.

---

## Code Lowering Comparison

The following example demonstrates a Bell pair circuit written in Naso, transformed into OpenQASM 3.0, and compiled down to QIR Bitcode:

{{< tabs items="Naso,OpenQASM 3.0,QIR Bitcode" >}}
  {{< tab >}}
```naso
fn create_bell_pair(inout q0: [1] Qubit, inout q1: [1] Qubit) -> [0] Proof {
    hadamard(inout q0);
    cnot(inout q0, inout q1);
}
```
  {{< /tab >}}
  {{< tab >}}
```openqasm3
// OpenQASM 3.0 code for Bell pair
OPENQASM 3;
include "stdgates.inc";
qubit[2] q;
h q[0];
cx q[0], q[1];
```
  {{< /tab >}}
  {{< tab >}}
```llvm
; QIR Bitcode (simplified)
define void @create_bell_pair(%Qubit* %q0, %Qubit* %q1) {
entry:
  call void @quantum%qbit%hadamard(%Qubit* %q0)
  call void @quantum%qbit%cx(%Qubit* %q0, %Qubit* %q1)
  ret void
}
```
  {{< /tab >}}
{{< /tabs >}}

<div class="naso-diagnostic naso-diagnostic-error">
  Error: Linear variable `q0` used twice in scope.
</div>

## Verification with naso-verify

Naso's integrated SMT checker (`naso-verify`) validates QTT constraints at compile time. For the Bell pair example, it proves:

1. Each `[1] Qubit` is consumed exactly once (no leaks, no duplication)
2. The intermediate Hadamard state is uncomputed before scope exit (Bennett compliance)
3. The final `[0] Proof` is erased during codegen (zero runtime overhead)

Try verifying this function:
```naso
verify create_bell_pair
```