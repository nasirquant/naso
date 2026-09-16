# Naso QTT Type System — Formal Type Rules (Sprint 2)

## Notation

```
Γ   ::= ∅ | Γ, x : (τ @ q @ m)       // Type environment: variable → (type, quantity, mutability)
Δ   ::= ∅ | Δ, α : κ                  // Kind environment
Σ   ::= ∅ | Σ, f : ∀α. (τ₁ @ q₁ @ m₁, ..., τₙ @ qₙ @ mₙ) → τ @ q @ m  // Function signatures
Ξ   ::= ∅ | Ξ, ?M : τ                  // Metavariable store
C   ::= ∅ | C, q₁ ≤ q₂                // Quantity constraints
```

**Quantities**: q ∈ {0, 1, N (bounded), * (many)} with subtyping: 0 ≤ 1 ≤ N ≤ *
**Mutabilities**: m ∈ {imm, inout, consume}

**Judgments**:
- Γ; Δ; Σ ⊢ₑ e ⇐ τ @ q @ m     // Check: expression e checks against expected type τ with quantity q, mutability m
- Γ; Δ; Σ ⊢ₑ e ⇒ τ @ q @ m     // Infer: expression e synthesizes type τ with quantity q, mutability m
- Γ ⊢ q₁ ≤ q₂                  // Quantity subtyping
- Γ ⊢ τ₁ ≡ τ₂                  // Type equivalence (with quantity)
- Γ ⊢ e ⇓ v                    // Evaluation (for [0] erasure verification)

---

## Quantity Subtyping & Arithmetic

```
────────────────────── (Q-REFL)
Γ ⊢ q ≤ q

────────────────────── (Q-ZERO)
Γ ⊢ 0 ≤ q

────────────────────── (Q-ONE)
Γ ⊢ 1 ≤ q       if q ∈ {1, N, *}

────────────────────── (Q-BOUNDED)
Γ ⊢ N₁ ≤ N₂     if N₁ ≤ N₂ (as naturals)

────────────────────── (Q-MANY)
Γ ⊢ q ≤ *       for all q
```

**Quantity join (for merges in control flow):**
```
q₁ ⊔ q₂ = max(q₁, q₂)  under lattice 0 < 1 < N < *
```

**Quantity consumption (use-site):**
```
use(q) = 
  0        if q = 0
  1        if q = 1
  N-1      if q = N (N > 1)
  *        if q = *
```

---

## Bidirectional Typing Rules

### Variables & Literals

```
x : (τ @ q @ m) ∈ Γ
────────────────────────────────────────── (VAR-CHECK)
Γ ⊢ₑ x ⇐ τ @ q @ m

x : (τ @ q @ m) ∈ Γ
────────────────────────────────────────── (VAR-INFER)
Γ ⊢ₑ x ⇒ τ @ q @ m
```

**Variable usage tracking**: On every VAR rule application, increment usage counter for x in Γ.
- If q = 1 and counter > 1: **Error** "linear variable used multiple times"
- If q = 0 and in runtime position: **Error** "erased variable used at runtime"

```
────────────────────────────────────────── (LIT-CHECK)
Γ ⊢ₑ n ⇐ Int @ * @ imm     // Integer literal

────────────────────────────────────────── (LIT-INFER)
Γ ⊢ₑ n ⇒ Int @ * @ imm
```

(Same for Float, Bool, String, Char, Unit)

---

### Let Bindings

```
Γ ⊢ₑ e₁ ⇒ τ₁ @ q₁ @ m₁      Γ, x : (τ₁ @ q₁ @ m₁) ⊢ₑ e₂ ⇐ τ₂ @ q₂ @ m₂
────────────────────────────────────────────────────────────────────────── (LET-CHECK)
Γ ⊢ₑ let x = e₁; e₂ ⇐ τ₂ @ q₂ @ m₂

Γ ⊢ₑ e₁ ⇒ τ₁ @ q₁ @ m₁      Γ, x : (τ₁ @ q₁ @ m₁) ⊢ₑ e₂ ⇒ τ₂ @ q₂ @ m₂
────────────────────────────────────────────────────────────────────────── (LET-INFER)
Γ ⊢ₑ let x = e₁; e₂ ⇒ τ₂ @ q₂ @ m₂
```

**With explicit quantity/mutability annotation:**
```
Γ ⊢ₑ e₁ ⇐ τ₁ @ q @ m        Γ, x : (τ₁ @ q @ m) ⊢ₑ e₂ ⇐ τ₂ @ q₂ @ m₂
────────────────────────────────────────────────────────────────────────── (LET-ANN-CHECK)
Γ ⊢ₑ let [q] m x: τ₁ = e₁; e₂ ⇐ τ₂ @ q₂ @ m₂
```

**Quantity enforcement at binding:**
- If annotation provides q, check q₁ ≤ q (value quantity ≤ declared)
- If q = 0: verify e₁ is **erasable** (no side effects, no quantum ops, no I/O)
- If q = 1: x must be consumed exactly once in e₂ (enforced by usage counter)

---

### InOut (Mutable Projection) Bindings

```
Γ ⊢ₑ e₁ ⇒ τ₁ @ 1 @ imm        Γ, x : (τ₁ @ 1 @ inout) ⊢ₑ e₂ ⇐ τ₂ @ q₂ @ m₂
────────────────────────────────────────────────────────────────────────────────── (LET-INOUT-CHECK)
Γ ⊢ₑ let inout x = e₁; e₂ ⇐ τ₂ @ q₂ @ m₂

Γ ⊢ₑ e₁ ⇒ τ₁ @ 1 @ imm        Γ, x : (τ₁ @ 1 @ inout) ⊢ₑ e₂ ⇒ τ₂ @ q₂ @ m₂
────────────────────────────────────────────────────────────────────────────────── (LET-INOUT-INFER)
Γ ⊢ₑ let inout x = e₁; e₂ ⇒ τ₂ @ q₂ @ m₂
```

**InOut invariants (enforced in type_env):**
1. `e₁` must be a **place expression** (variable, field access, index) — not a temporary
2. `τ₁` must have quantity 1 (unique ownership required for mutation)
3. During `e₂`, **no other binding** (imm, inout, consume) may alias `x`
4. At end of `e₂`, `x` is **re-projected** back to imm with same type

**Aliasing check** (pseudo-code in type_env):
```rust
fn bind_inout(&mut self, x: Ident, place: Place) {
    // 1. Verify place has quantity 1
    // 2. Check no existing binding overlaps with place
    // 3. Mark place as "borrowed inout" — blocks all other access
    // 4. On scope exit: restore place to imm, clear borrow flag
}
```

---

### Consume (Linear Move) Bindings

```
Γ ⊢ₑ e₁ ⇒ τ₁ @ 1 @ imm        Γ, x : (τ₁ @ 1 @ consume) ⊢ₑ e₂ ⇐ τ₂ @ q₂ @ m₂
────────────────────────────────────────────────────────────────────────────────── (LET-CONSUME-CHECK)
Γ ⊢ₑ let consume x = e₁; e₂ ⇐ τ₂ @ q₂ @ m₂
```

**Consume semantics:**
- `x` has mutability `consume` — **exactly one use** allowed in `e₂`
- After that use, `x` is **invalidated** in Γ (removed or marked `moved`)
- Any subsequent use of `x` → **Error** "use of moved value"
- At scope exit: all `consume` bindings must be used (linear consumption enforced)

---

### Function Abstraction (Lambda)

```
Γ, x₁:(τ₁@q₁@m₁), ..., xₙ:(τₙ@qₙ@mₙ) ⊢ₑ e ⇐ τ @ q @ m
────────────────────────────────────────────────────────────────────────── (LAMBDA-CHECK)
Γ ⊢ₑ λ(x₁:τ₁@q₁@m₁, ..., xₙ:τₙ@qₙ@mₙ). e ⇐ (τ₁@q₁@m₁, ..., τₙ@qₙ@mₙ) → τ @ q @ m @ * @ imm

Γ, x₁:(τ₁@q₁@m₁), ..., xₙ:(τₙ@qₙ@mₙ) ⊢ₑ e ⇒ τ @ q @ m
────────────────────────────────────────────────────────────────────────── (LAMBDA-INFER)
Γ ⊢ₑ λ(x₁:τ₁@q₁@m₁, ..., xₙ:τₙ@qₙ@mₙ). e ⇒ (τ₁@q₁@m₁, ..., τₙ@qₙ@mₙ) → τ @ q @ m @ * @ imm
```

**Capture semantics** (for closures):
- Each captured variable gets a `CaptureMode`: ByValue, ByInOut, ByConsume
- ByValue: requires q = * (copyable)
- ByInOut: requires q = 1, no aliasing
- ByConsume: requires q = 1, moves the value

---

### Function Application

```
Γ ⊢ₑ e₁ ⇒ (τ₁@q₁@m₁, ..., τₙ@qₙ@mₙ) → τ @ q @ m @ qf @ mf
Γ ⊢ₑ eᵢ ⇐ τᵢ @ qᵢ @ mᵢ        for all i ∈ 1..n
────────────────────────────────────────────────────────────────────────── (APP-CHECK)
Γ ⊢ₑ e₁(e₂, ..., eₙ₊₁) ⇐ τ @ q @ m

Γ ⊢ₑ e₁ ⇒ (τ₁@q₁@m₁, ..., τₙ@qₙ@mₙ) → τ @ q @ m @ qf @ mf
Γ ⊢ₑ eᵢ ⇒ τᵢ' @ qᵢ' @ mᵢ'       τᵢ' ≡ τᵢ @ qᵢ @ mᵢ   (with qty subtyping)
────────────────────────────────────────────────────────────────────────── (APP-INFER)
Γ ⊢ₑ e₁(e₂, ..., eₙ₊₁) ⇒ τ @ q @ m
```

**Argument passing rules:**
- `mᵢ = imm`: argument passed by value (requires qᵢ' ≤ qᵢ, copy if qᵢ = *)
- `mᵢ = inout`: argument must be a place with qᵢ' = 1, no aliasing
- `mᵢ = consume`: argument must have qᵢ' = 1, value moved (invalidated after call)

---

### If Expression

```
Γ ⊢ₑ e₁ ⇐ Bool @ * @ imm      Γ ⊢ₑ e₂ ⇐ τ @ q₂ @ m₂      Γ ⊢ₑ e₃ ⇐ τ @ q₃ @ m₃
────────────────────────────────────────────────────────────────────────────────────── (IF-CHECK)
Γ ⊢ₑ if e₁ { e₂ } else { e₃ } ⇐ τ @ (q₂ ⊔ q₃) @ (m₂ ⊔ m₃)

Γ ⊢ₑ e₁ ⇒ Bool @ * @ imm      Γ ⊢ₑ e₂ ⇒ τ₂ @ q₂ @ m₂      Γ ⊢ₑ e₃ ⇒ τ₃ @ q₃ @ m₃      τ₂ ≡ τ₃
──────────────────────────────────────────────────────────────────────────────────────────────── (IF-INFER)
Γ ⊢ₑ if e₁ { e₂ } else { e₃ } ⇒ τ₂ @ (q₂ ⊔ q₃) @ (m₂ ⊔ m₃)
```

---

### Match Expression

```
Γ ⊢ₑ e ⇒ τ₀ @ q₀ @ m₀
∀i. Γ ⊢ₚ pᵢ ⇐ τ₀ @ q₀ @ mᵢ ⊢ Γᵢ      Γ, Γᵢ ⊢ₑ eᵢ ⇐ τ @ qᵢ @ mᵢ'
────────────────────────────────────────────────────────────────────────────── (MATCH-CHECK)
Γ ⊢ₑ match e { p₁ => e₁, ..., pₙ => eₙ } ⇐ τ @ (⊔ᵢ qᵢ') @ (⊔ᵢ mᵢ')
```

**Pattern typing** (⊢ₚ):
- Patterns bind variables with their quantities from the scrutinee type
- `x @ q` in pattern binds `x : (τ @ q @ imm)`
- `inout x @ 1` binds `x : (τ @ 1 @ inout)`
- `consume x @ 1` binds `x : (τ @ 1 @ consume)`

---

### Reversible Blocks

```
Γ ⊢ₑ eᵢ ⇐ τᵢ @ qᵢ @ mᵢ        (pure check: no I/O, no measure, no entangle)
∀i. qᵢ = 0 ∨ (qᵢ = 1 ∧ has_inverse(eᵢ))
────────────────────────────────────────────────────────────────────────────────────── (REV-CHECK)
Γ ⊢ₑ reversible { e₁; ...; eₙ } ⇐ Unit @ * @ imm
```

**Purity restrictions inside `reversible`:**
- No `measure`, `entangle`, `gate` application (quantum ops with side effects)
- No I/O, no allocation of linear resources
- All `[1]` bindings must have registered `UncomputeStep` inverses
- `[0]` bindings are **erased** — verified by checking `e ⇓ v` at compile time

**Uncomputation DAG validation:**
- Each `UncomputeStep { target, inverse_expr, deps }` forms a DAG edge `deps → target`
- Check: no cycles, all `deps` defined before `target`, all targets have inverses

---

### Quantum Operations

**Qubit Allocation:**
```
────────────────────────────────────────── (QUBIT-ALLOC)
Γ ⊢ₑ qubit x ⇒ Qubit @ 1 @ imm
```
- Produces fresh linear qubit (quantity 1)

**Measurement:**
```
Γ ⊢ₑ e ⇐ Qubit @ 1 @ consume
────────────────────────────────────────── (MEASURE)
Γ ⊢ₑ measure e ⇒ Bool @ 1 @ imm
```
- Consumes qubit (linear use), produces classical bit

**Gate Application:**
```
Γ ⊢ₑ e₁ ⇒ Qubit @ 1 @ inout      Γ ⊢ₑ e₂ ⇐ Param @ * @ imm      ...
──────────────────────────────────────────────────────────────────── (GATE-APPLY)
Γ ⊢ₑ gate G(e₁, e₂, ...) ⇒ Unit @ * @ imm
```
- Qubit passed as `inout` (mutable, no aliasing)
- Parameters are classical (`*`)

**Entanglement:**
```
Γ ⊢ₑ eᵢ ⇐ Qubit @ 1 @ consume    for all i
────────────────────────────────────────────────────────────────────────── (ENTANGLE)
Γ ⊢ₑ entangle(e₁, ..., eₙ) ⇒ QRegister[n] @ 1 @ imm
```
- All qubits consumed, produces linear register

---

### Dependent Types (Pi / Sigma)

**Pi Type Formation:**
```
Γ, x:τ₁ ⊢ τ₂ type
────────────────────────────────────────── (PI-FORM)
Γ ⊢ Π(x:τ₁). τ₂ type
```

**Pi Introduction (dependent function):**
```
Γ, x:(τ₁@q₁@m₁) ⊢ₑ e ⇐ τ₂ @ q₂ @ m₂
────────────────────────────────────────────────────────────────── (PI-INTRO)
Γ ⊢ₑ λ(x:τ₁@q₁@m₁). e ⇐ Π(x:τ₁@q₁@m₁). τ₂ @ * @ imm
```

**Pi Elimination (application):**
```
Γ ⊢ₑ e₁ ⇒ Π(x:τ₁@q₁@m₁). τ₂ @ q @ m      Γ ⊢ₑ e₂ ⇐ τ₁ @ q₁ @ m₁
──────────────────────────────────────────────────────────────────────────────────── (PI-ELIM)
Γ ⊢ₑ e₁(e₂) ⇒ τ₂[e₂/x] @ q₂ @ m₂
```

**Sigma Type (dependent pair):**
```
Γ ⊢ₑ e₁ ⇒ τ₁ @ q₁ @ m₁      Γ ⊢ₑ e₂ ⇐ τ₂[e₁/x] @ q₂ @ m₂
────────────────────────────────────────────────────────────────────────── (SIGMA-INTRO)
Γ ⊢ₑ (e₁, e₂) ⇒ Σ(x:τ₁@q₁@m₁). τ₂ @ (q₁ ⊔ q₂) @ (m₁ ⊔ m₂)

Γ ⊢ₑ e ⇒ Σ(x:τ₁@q₁@m₁). τ₂ @ q @ m      Γ, x:(τ₁@q₁@m₁), y:(τ₂@q₂@m₂) ⊢ₑ e' ⇐ τ @ q' @ m'
────────────────────────────────────────────────────────────────────────────────────────────────── (SIGMA-ELIM)
Γ ⊢ₑ match e { (x, y) => e' } ⇐ τ @ q' @ m'
```

---

### Type Ascription

```
Γ ⊢ₑ e ⇐ τ @ q @ m
────────────────────────────────────────── (ASCR-CHECK)
Γ ⊢ₑ (e : τ @ q @ m) ⇐ τ @ q @ m

Γ ⊢ₑ e ⇒ τ' @ q' @ m'      τ' ≡ τ @ q @ m
────────────────────────────────────────── (ASCR-INFER)
Γ ⊢ₑ (e : τ @ q @ m) ⇒ τ @ q @ m
```

---

## Erasure Verification (Compile-Time)

For every binding `x : (τ @ 0 @ m)`:
1. Verify `x` only appears in **erasable positions**:
   - Type annotations
   - Proof terms (in `reversible` bodies)
   - Compile-time `NatExpr` computation
2. Verify `x` **never appears in runtime positions**:
   - Function arguments (unless callee expects [0])
   - Return values
   - Field values in structs
   - Quantum operations
   - Branch conditions (unless branch is also [0])

**Erasure check algorithm:**
```rust
fn verify_erasure(expr: &Expr, env: &TypeEnv) -> Result<(), Error> {
    match expr.kind {
        ExprKind::Var(x) if env.get(x).quantity == Quantity::Zero => {
            if !is_erasable_position(expr) {
                Err(Error::ErasedVariableUsedAtRuntime { span: expr.span })
            }
        }
        ExprKind::Call(fun, args) => {
            for (i, arg) in args.iter().enumerate() {
                if fun.param(i).quantity == Quantity::Zero {
                    verify_erasure(arg, env)?
                }
            }
        }
        // ... recurse into subexpressions
    }
    Ok(())
}
```

---

## Metavariable & Constraint Solving

**Metavariable creation:**
```
fresh_metavar(τ) → ?M
Ξ[?M] = τ
```

**Constraint generation (during inference):**
- Type equality: `τ₁ ≡ τ₂` → decompose to sub-constraints
- Quantity constraint: `q₁ ≤ q₂` → add to C
- Mutability constraint: `m₁ ≤ m₂` (imm ≤ inout ≤ consume? No — they're distinct modes)

**Solving order:**
1. Unify rigid-rigid (user-annotated) types
2. Solve quantity constraints (lattice)
3. Instantiate metavariables from constraints
4. Check occurs check for recursive types
5. Generalize remaining metavariables (quantify)

---

## Error Reporting Format

```
error[E0308]: quantity mismatch
  --> src/main.naso:12:15
   |
12 |     let x: [1] Int = 42;
   |               ^^^ expected `[1]`, found `[*]`
   |
   = note: literal `42` has quantity `[*]` (unrestricted)
   = help: remove quantity annotation or use `consume 42` to move

error[E0502]: inout aliasing violation
  --> src/main.naso:23:9
   |
22 |     let inout x = &mut data;
23 |     let y = &data;  // immutable borrow aliases inout
   |     ^^^^^^^^^^^^^ immutable borrow occurs here
   |
   = note: `inout` projection requires unique access

error[E0382]: use of moved value
  --> src/main.naso:45:13
   |
44 |     let consume q = allocate_qubit();
45 |     let r = measure q;
   |             ^ value moved here
46 |     let s = measure q;  // second use
   |             ^ value used again after move
   |
   = note: `consume` bindings are linear — use exactly once
```

---

## Integration with AST

**TypeEnv extensions needed:**
```rust
struct TypeEnv {
    vars: IndexMap<Ident, VarInfo>,
    types: IndexMap<Ident, TypeDef>,
    quantities: IndexMap<Ident, Quantity>,
    // NEW:
    usage_counts: IndexMap<Ident, u32>,        // for [1] tracking
    inout_borrows: IndexMap<Ident, Place>,     // active inout projections
    moved_vars: HashSet<Ident>,                // consumed variables
    erasable_vars: HashSet<Ident>,             // [0] variables
}

struct VarInfo {
    ty: Type,
    quantity: Quantity,
    mutability: Mutability,
    defined_at: Span,
    used_at: Vec<Span>,
}
```

---

## Next Steps

1. **TASK-201**: Create `compiler/src/typecheck/` module structure with stubs
2. **TASK-202**: Implement `TypeEnv` with quantity/mutability/usage tracking
3. **TASK-203/204**: Implement `check.rs` and `inference.rs` with rules above
4. **TASK-205**: Quantity unification & constraint solving
5. **TASK-206/207**: InOut aliasing + Consume move checking
6. **TASK-208**: Reversible block purity + uncomputation DAG
7. **TASK-209**: Quantum-specific linearity rules
8. **TASK-210**: Comprehensive test suite