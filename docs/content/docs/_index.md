---
title: "Naso Documentation"
weight: 1
---

# Naso Language Documentation

This documentation covers the Naso programming language, a systems language integrating **Quantitative Type Theory (QTT)**, **Mutable Value Semantics (MVS)**, **automatic uncomputation**, and **polyhedral cross-hardware compilation** with an embedded **Z3 SMT prover**.

## Documentation Structure

| Section | Description |
|---------|-------------|
| [Language Specification](/docs/spec/) | Complete formal grammar, type rules, and operational semantics |
| [Quantitative Type Theory](/docs/qtt/) | Deep dive into QTT quantities, semiring, and typing judgments |
| [Standard Library](/docs/stdlib/) | API reference for `std::quantum`, `std::poly`, `std::smt`, `std::mem`, `std::io` |
| [Formal Verification](/docs/verification/) | `naso-verify` SMT engine, error codes, and soundness proofs |

## Quick Start

```bash
# Install Naso toolchain
curl --proto '=https' --tlsv1.2 -sSf https://nasolang.org/install.sh | sh

# Create a new project
naso new hello_naso
cd hello_naso

# Build and run
naso run
```

## Hello World: Bell Pair

```naso
fn main() -> [0] Proof {
    let q0 = qalloc();
    let q1 = qalloc();
    hadamard(inout q0);
    cnot(inout q0, inout q1);
    // q0, q1 now form a Bell pair |00⟩ + |11⟩
    // Automatic uncomputation on scope exit
}
```

## Core Concepts at a Glance

| Concept | Notation | Runtime Behavior |
|---------|----------|------------------|
| **Erased/Proof** | `[0] T` | Fully eliminated during codegen |
| **Linear/Unique** | `[1] T` | Must be consumed exactly once |
| **Classical/Unbounded** | `[*] T` | Unlimited copies, implicit drops |
| **Bounded** | `[N] T` | Static quantity bounded by polyhedral domain |
| **Mutable Reference** | `inout x: [1] T` | Exclusive write access, copy-in/copy-out |

## Next Steps

- Start with the [Language Specification](/docs/spec/) for formal grammar and semantics
- Learn [Quantitative Type Theory](/docs/qtt/) for resource-aware programming
- Explore the [Standard Library](/docs/stdlib/) for quantum and polyhedral APIs
- Understand [Formal Verification](/docs/verification/) for provably correct code