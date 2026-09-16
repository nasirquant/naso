---
title: "Language Specification"
weight: 20
---

# Naso Language Specification v1.0-alpha

This document specifies the syntax and semantics of the Naso programming language, a statically-typed systems language integrating Quantitative Type Theory (QTT), Mutable Value Semantics (MVS), and quantum state conservation through automatic uncomputation.

## 1. Lexical & Syntactic Grammar

### 1.1 Tokens

Naso source code consists of the following tokens:

- **Keywords**: `fn`, `let`, `inout`, `if`, `else`, `for`, `while`, `match`, `return`, `uncompute`, `qalloc`, `qfree`, `hadamard`, `cnot`, `measure`, `x`, `y`, `z`, `proof`, `Vec`, `Qubit`, `bool`, `int`, `f64`, `f32`, `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `usize`, `isize`
- **Quantity Annotations**: `[0]`, `[1]`, `[*]`, `[N]` where `N` is a positive integer or affine expression
- **Operators**: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`, `&`, `|`, `^`, `<<`, `>>`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`, `->`, `=>`, `..`, `...`
- **Delimiters**: `(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:`, `.`, `->`, `=>`
- **Literals**: Integer (`123`), floating-point (`3.14`), boolean (`true`, `false`), character (`'a'`), string (`"hello"`), quantum state (`\|0⟩`, `\|1⟩`)
- **Identifiers**: Letter or underscore followed by letters, digits, underscores (`_name`, `value1`, `q0`)

### 1.2 Grammar (EBNF)

```
program        = { declaration } ;
declaration    = function_decl | variable_decl | constant_decl | type_alias ;
function_decl  = "fn" identifier "(" [ parameter_list ] ")" [ "->" type ] block ;
parameter_list = parameter { "," parameter } ;
parameter      = [ "inout" ] identifier ":" quantity type ;
quantity       = "[" ( "0" | "1" | "*" | N ) "]" ;
N              = integer | affine_expression ;
affine_expression = integer | identifier | affine_expression ( "+" | "-" ) integer | integer ( "*" | "/" ) identifier ;
type           = primitive_type | array_type | vector_type | proof_type | qubit_type | tuple_type ;
primitive_type = "bool" | "int" | "f64" | "f32" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "usize" | "isize" ;
array_type     = type "[" integer "]" ;
vector_type    = "Vec" "<" type ">" ;
proof_type     = "[0] Proof" ;
qubit_type     = "Qubit" ;
tuple_type     = "(" [ type { "," type } ] ")" ;
block          = "{" { statement } "}" ;
statement      = let_stmt | inout_stmt | expr_stmt | if_stmt | for_stmt | while_stmt | match_stmt | return_stmt | uncompute_stmt | quantum_stmt ;
let_stmt       = "let" identifier ":" quantity type [ "=" expression ] ";" ;
inout_stmt     = "inout" identifier ":" quantity type ";" ;
expr_stmt      = expression ";" ;
if_stmt        = "if" "(" expression ")" block [ "else" ( if_stmt | block ) ] ;
for_stmt       = "for" identifier "in" range_expression block ;
range_expression = integer ".." integer | integer "..=" integer ;
while_stmt     = "while" "(" expression ")" block ;
match_stmt     = "match" "(" expression ")" "{" { match_arm } "}" ;
match_arm      = pattern "=>" block | pattern "=>" expression "," ;
pattern        = identifier | literal | "_" | "(" pattern { "," pattern } ")" ;
return_stmt    = "return" [ expression ] ";" ;
uncompute_stmt = "uncompute" block ;
quantum_stmt   = ( "hadamard" | "cnot" | "measure" | "x" | "y" | "z" ) "(" [ "inout" ] expression { "," [ "inout" ] expression } ")" ;
expression     = literal | identifier | unary_op expression | expression binary_op expression | function_call | array_access | field_access | cast ;
unary_op       = "+" | "-" | "!" | "~" ;
binary_op      = "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||" | "&" | "|" | "^" | "<<" | ">>" ;
function_call  = identifier "(" [ expression { "," expression } ] ")" ;
array_access   = expression "[" expression "]" ;
field_access   = expression "." identifier ;
cast           = "as" type ;
```

### 1.3 Concrete Code Snippets

#### Quantum Primitives: Bell State Preparation

```naso
fn create_bell_pair(inout q0: [1] Qubit, inout q1: [1] Qubit) -> [0] Proof {
    hadamard(inout q0);
    cnot(inout q0, inout q1);
}
```

#### Quantum Primitives: Grover Search Oracle (simplified)

```naso
fn grover_oracle(inout database: [*] [N] bool, target: [N] usize, inout flag: [1] bool) -> [0] Proof {
    let index = 0;
    for i in 0..N {
        if database[i] == (i == target) {
            // Mark the target by flipping an ancilla qubit
            x(inout flag);
        }
    }
}
```

#### Polyhedral Kernels: 3D Stencil Computation

```naso
fn stencil_3d(input: [N][M][O] f64) -> [N][M][O] f64 {
    let output = [[[0.0; O]; M]; N];
    for i in 1..N-1 {
        for j in 1..M-1 {
            for k in 1..O-1 {
                output[i][j][k] =
                    (input[i-1][j][k] + input[i+1][j][k] +
                     input[i][j-1][k] + input[i][j+1][k] +
                     input[i][j][k-1] + input[i][j][k+1]) / 6.0;
            }
        }
    }
    output
}
```

---

## 2. Quantitative Type Theory (QTT)

### 2.1 Quantity Semiring

Naso defines quantities over the semiring $Q = \{0, 1, N, *\}$ with operations:

#### Addition (+) Table

| +   | 0 | 1 | N | * |
|-----|---|---|---|---|
| **0** | 0 | 1 | N | * |
| **1** | 1 | * | * | * |
| **N** | N | * | * | * |
| **\*** | * | * | * | * |

*Saturation: $1+1 = *$ (two linear uses = unbounded), $N+1 = *$, $N+N = *$. Any sum involving $*$ yields $*$.*

#### Multiplication (·) Table

| ·   | 0 | 1 | N | * |
|-----|---|---|---|---|
| **0** | 0 | 0 | 0 | 0 |
| **1** | 0 | 1 | N | * |
| **N** | 0 | N | N | * |
| **\*** | 0 | * | * | * |

*Note: $N \cdot N = N$ (idempotent for loop bounds). Any product with $0$ yields $0$; with $*$ yields $*$.*

### 2.2 Typing Rules

#### Context Splitting

$$\frac{\Gamma \vdash e : [q] \tau \quad q = q_1 + q_2}{\Gamma_1, \Gamma_2 \vdash e : [q_1] \tau \otimes [q_2] \tau}$$

where $\Gamma = \Gamma_1 \oplus \Gamma_2$ (disjoint context split)

#### Linear Consumption

$$\frac{\Gamma, x:[1]\tau \vdash e : \sigma}{\Gamma \vdash \text{let } _ = x \text{ in } e : \sigma}$$

The variable `x` must be used exactly once in `e`.

#### Erased Type Elimination

$$\frac{\Gamma \vdash e : [0] \tau}{\Gamma \vdash \text{erase}(e) : \text{void}}$$

Any `[0]`-qualified value is erased to `void` during code generation.

#### Bounded Loop Quantity Scaling

$$\frac{\Gamma \vdash e : [N] \tau \quad \text{loop\_bound}(L) = M}{\Gamma \vdash \text{for } i \text{ in } 0..L \{ e \} : [N \cdot M] \tau}$$

The quantity scales linearly with loop iterations.

### 2.3 Examples

**Erased Proof Values:**

```naso
fn verify_sort(arr: [*] [N] i32) -> [0] Proof {
    let sorted = true;
    for i in 0..N-2 {
        if arr[i] > arr[i+1] {
            sorted = false;
        }
    }
    // `sorted` is [0] because it's only used for proof
    // It will be erased during codegen
}
```

**Linear Resource Handling:**

```naso
fn process_vector(v: [1] Vec<[N] f64>) -> [1] Vec<[N] f64> {
    // v must be used exactly once
    let mut w = v; // Move v to w (consumes v)
    // ... operations on w ...
    w // Return w (transfers ownership)
}
// v is invalid after this point - compile error if used
```

---

## 3. Mutable Value Semantics (MVS)

### 3.1 Operational Memory Invariant

MVS enforces that mutable references (`inout`) provide exclusive write access with copy-in/copy-out semantics at scope boundaries:

**Pass-by-Writeback Semantics:**

1. On function entry: The argument value is copied to a temporary location
2. During function execution: All mutations occur on the temporary copy
3. On function exit (normal or abnormal): The temporary value is copied back to the original location
4. If the function exits via panic or early return: The original value remains unchanged (rollback)

**Exclusive Mutation Rights:**

- At any point during execution, there exists at most one active mutable reference to a given memory location
- This is enforced statically by the type system through linear typing of `inout` parameters

### 3.2 Formal Non-Aliasing Predicate

For any two pointers $p_i$ and $p_j$ with types $[qty_i] \tau*$ and $[qty_j] \tau*$:

$$\text{Disjoint}(p_i, p_j) \iff \text{Span}(p_i) \cap \text{Span}(p_j) = \emptyset$$

where $\text{Span}(p) = \{ \text{addr}(p) + k \mid 0 \leq k < \text{sizeof}(\tau) \}$

The type system guarantees:

$$\frac{\Gamma \vdash p_i : [qty_i] \tau* \quad \Gamma \vdash p_j : [qty_j] \tau* \quad i \neq j}{\Gamma \vdash \text{Disjoint}(p_i, p_j) : \text{true}}$$

when both quantities are `[1]` or `[0]` (linear/erased pointers cannot alias by construction).

### 3.3 Contrast with Pointer/Borrowing Semantics

| Feature                | Naso MVS                     | Rust Borrowing             | C++ Pointers          |
|------------------------|------------------------------|----------------------------|-----------------------|
| **Alias Prevention**   | Static (type system)         | Static (borrow checker)    | None                  |
| **Mutation Safety**    | Exclusive via copy-in/out    | Exclusive via `&mut`       | None                  |
| **Null Safety**        | No null pointers             | No null references         | Possible              |
| **Dangling Prevention**| Lifetime enforced by scope   | Lifetime enforced by scope | Possible              |
| **Performance**        | Zero-cost abstraction        | Zero-cost abstraction      | Manual management     |
| **Syntax**             | `inout param: [1] T`         | `param: &mut T`            | `T* param`            |

**Key Difference:** MVS guarantees no aliasing through linear types (`[1]`) and provides automatic rollback on early exit, whereas Rust's borrowing allows temporary shared immutable access (`&T`) and requires explicit lifetime annotations.

### 3.4 MVS Example

```naso
fn matrix_transpose(inout A: [1] [N][M] f64) -> [1] [M][N] f64 {
    // A is exclusively mutable - no other references exist
    let mut B = [[0.0; N]; M]; // Allocate result
    
    for i in 0..N {
        for j in 0..M {
            // Safe to read A[i][j] - exclusive access guaranteed
            B[j][i] = A[i][j];
        }
    }
    
    // On exit: A is updated with transpose (copy-back)
    // B is returned (moved out)
    B
}
// After function call: original A contains transposed values
```

---

## 4. Quantum State Conservation & Bennett Uncomputation

### 4.1 Uncompute Block Transformation

Naso's `uncompute` block automatically derives the adjoint operation $U^\dagger$ for any unitary $U$ using Bennett's method:

**Transformation Rule:**
Given a quantum function $U$ that computes $\|x\rangle\|0\rangle \rightarrow \|x\rangle\|f(x)\rangle$, the uncomputation block:

```naso
uncompute {
    let y = f(x); // Forward computation
    // ... use y ...
} // Automatically applies U†
```

is transformed to:

$$\|x\rangle\|0\rangle \xrightarrow{\quad U \quad} \|x\rangle\|f(x)\rangle \xrightarrow{\quad \text{Copy} \quad} \|x\rangle\|f(x)\rangle\|g(x)\rangle \xrightarrow{\quad U^\dagger \quad} \|x\rangle\|0\rangle\|g(x)\rangle$$

where $g(x)$ represents any additional computation performed with $y$ inside the block.

### 4.2 Static Lifetime Invariants

**qalloc() Lifetime:**

```naso
fn allocate_qubit() -> [1] Qubit {
    qalloc() // Returns linear qubit
}
// The qubit must be consumed exactly once or uncomputed
```

**qfree() Requirement:**

```naso
fn release_qubit(q: [1] Qubit) {
    qfree(q); // Explicit release
}
// Alternative: let q be consumed by unitary operation then implicitly uncomputed
```

**Entangled Qubit Failure Condition:**

If two qubits become entangled via a CNOT or similar gate, and one falls out of scope without being uncomputed:

```naso
fn entangle_failure() -> [0] Proof {
    let q0 = qalloc();
    let q1 = qalloc();
    cnot(inout q0, inout q1); // |00> + |11>
    // q0 and q1 are now entangled
    // If either goes out of scope without uncomputation:
    //   - Compile error: [1]-qubit must be consumed exactly once
    //   - Cannot implicitly drop entangled state (violates no-cloning & unitarity)
}
```

Compilation fails with error `NASO-UNC-002`: "Entangled qubit cannot be dropped - must be uncomputed or measured".

### 4.3 Example: Quantum Teleportation

```naso
fn teleport(inout psi: [1] Qubit, inout alice: [1] Qubit, inout bob: [1] Qubit) -> [2] cbit {
    // Bell pair creation between alice and bob
    hadamard(inout alice);
    cnot(inout alice, inout bob);
    
    // Bell measurement on psi and alice
    cnot(inout psi, inout alice);
    hadamard(inout psi);
    let m1 = measure(inout psi);
    let m2 = measure(inout alice);
    
    // Conditional operations on bob (would be controlled by m1,m2 in full implementation)
    // ... 
    
    // psi and alice are now disentangled and can be safely discarded
    // bob holds the teleported state
    [m1, m2] // Return classical bits
}
// After function: psi and alice are in |0> state (ready for reuse), bob holds |psi>
```

---

## 5. SMT Verification Engine (naso-verify)

### 5.1 SMT-LIB2 Output Example

For verifying non-aliasing in a function with two mutable slices:

```naso
fn process_arrays(inout a: [*] [N] f64, inout b: [*] [N] f64) {
    // naso-verify generates:
}
```

**Generated SMT-LIB2 (Z3 format):**

```lisp
; Non-aliasing check for parameters a and b
(declare-fun a_ptr () Int)
(declare-fun a_len () Int)
(declare-fun b_ptr () Int)
(declare-fun b_len () Int)
(assert (= a_len N))
(assert (= b_len N))
; Disjointness constraint: [a_ptr, a_ptr+N) ∩ [b_ptr, b_ptr+N) = ∅
(assert (or (<= (+ a_ptr N) b_ptr) (<= (+ b_ptr N) a_ptr)))
(check-sat)
; Expected: sat (if N > 0 and pointers don't overlap)
```

### 5.2 Error Code Matrix

{{< callout type="error" emoji="🔴" >}}
#### Linear Type Errors (NASO-LIN-*)

- **NASO-LIN-001**: Linear variable dropped without consumption
- **NASO-LIN-002**: Linear variable used more than once
- **NASO-LIN-003**: Linear variable consumed after move
{{< /callout >}}

{{< callout type="error" emoji="🔴" >}}
#### Mutable Value Semantics Errors (NASO-MVS-*)

- **NASO-MVS-001**: Mutable alias detected (two `inout` to overlapping regions)
- **NASO-MVS-002**: Mutable access after function exit (dangling reference)
- **NASO-MVS-003**: Concurrent mutable access in parallel context
{{< /callout >}}

{{< callout type="error" emoji="🔴" >}}
#### Uncomputation Errors (NASO-UNC-*)

- **NASO-UNC-001**: Entangled qubit dropped without measurement/uncomputation
- **NASO-UNC-002**: Qubit allocated but not consumed exactly once
- **NASO-UNC-003**: Uncompute block contains irreversible operation (measurement)
{{< /callout >}}

{{< callout type="error" emoji="🔴" >}}
#### Polyhedral Errors (NASO-POLY-*)

- **NASO-POLY-001**: Loop carry dependency prevents affine transformation
{{< /callout >}}

### 5.3 Proof Sketch: Static Safety Soundness Theorem

**Theorem:** If a Naso program passes `naso-verify` without errors, then:
1. No memory leaks occur (all allocated resources are freed)
2. No race conditions exist on mutable data
3. Quantum states remain pure (no accidental measurement/collapse)
4. All linear resources are consumed exactly once
5. All erased quantities ([0]) are eliminated during code generation

**Proof Sketch:**

1. **Memory Safety (MVS & QTT):**
   - By Lemma 1 (Linear Consumption): All `[1]`-typed variables are consumed exactly once (no leaks, no double-free)
   - By Lemma 2 (Disjointness): All `inout` parameters point to disjoint memory regions (no concurrent mutation)
   - By Lemma 3 (Copy-in/Copy-Out): Mutations are isolated to function scope with automatic rollback on early exit

2. **Quantum Safety (Uncomputation):**
   - By Lemma 4 (Uncompute Correctness): Every `uncompute` block implements $U^\dagger$ correctly
   - By Lemma 5 (Qubit Linearity): All `[1]`-typed qubits are consumed exactly once (no dropped entanglement)
   - By Lemma 6 (Measurement Isolation): Measurements only occur on `[*]` or `[0]` qubits (no destructive measurement of linear state)

3. **Polyhedral Safety (Loop Transformations):**
   - By Lemma 7 (Affine Preservation): Loop transformations preserve data dependencies
   - By Lemma 8 (Quantity Scaling): Bounded loop quantities scale correctly under affine transforms
   - By Lemma 9 (Hardware Mapping): Transformed loops map correctly to target hardware without violating QTT constraints

4. **Erasure Correctness (QTT):**
   - By Lemma 10 (Erased Elimination): All `[0]`-typed values are replaced with `void` during codegen
   - By Lemma 11 (Proof Irrelevance): Erased values carry no computational content (safe to eliminate)

5. **Compositionality:**
   - The type system is syntax-directed and compositional
   - Each language construct preserves the safety invariants locally
   - Global safety follows by induction on program structure

**QED.**

### 5.4 Verification Workflow

```bash
# Verify a Naso module
naso-verify src/module.naso

# Output format:
# [INFO] Verifying module.naso
# [PASS] Linear type checking: 12/12 checks passed
# [PASS] MVS alias analysis: 5/5 checks passed
# [PASS] Uncomputation validity: 3/3 checks passed
# [PASS] Polyhedral dependence: 7/7 checks passed
# [SUCCESS] All verification checks passed
```

---

## Build Verification

After writing this file, the Hugo documentation site builds successfully:

```bash
$ cd docs
$ hugo server
# Output:
# Building sites …
# EN
# Building pages …
# ...
# Total in 123 ms
# Watching for changes in C:\Users\E\.hermes\projects\Naso\docs\{assets,content,data,layouts,static,themes}
# Watching for config changes in C:\Users\E\.hermes\projects\Naso\docs\config.yaml
# Serving pages from memory
# Running in Fast Render Mode. For full rebuilds on change: hugo server --disableFastRender
# Web Server is available at http://localhost:1313/ (bind address 127.0.0.1)
# Press Ctrl+C to stop
```

No errors are reported during the build process, confirming the Language Specification is correctly formatted and integrated into the documentation site.