# tree-sitter-naso

Tree-sitter grammar for the Naso programming language.

## Overview

This repository contains the Tree-sitter grammar for [Naso](https://github.com/naso-lang/naso), a quantum-typed systems programming language with Quantitative Type Theory (QTT) linearity and Mutable Value Semantics (MVS).

The grammar provides syntax highlighting, code folding, and structural editing capabilities for Naso code in editors that support Tree-sitter (such as Neovim, Zed, and VS Code via the tree-sitter integration).

## Features

- Complete coverage of Naso syntax including:
  - QTT quantity annotations: `[0]`, `[1]`, `[*]`, `[N]`
  - Linear function signatures: `fn consume(x: [1] T)`
  - Inout parameters (MVS): `inout x: T`
  - Quantum intrinsics: `qalloc`, `hadamard`, `cnot`, `measure`, `bell_pair`, `qft`, `grover_oracle`
  - Tensor operations: `matmul`, `contract`, `transpose`, `outer_product`, `dot`
  - Polyhedral loop constructs: `forall`, `tile`, `fuse` with schedule annotations
  - Module and import system with prelude auto-import semantics
  - Structs, enums, generics, traits
  - Control flow: if/else, while, for, match
  - Expressions: literals, binary/unary operations, function calls
  - Uncomputation and defer statements
- High-performance incremental parsing
- Lossless Concrete Syntax Tree (CST) suitable for IDE features
- Prebuilt WASM parser for web usage
- Rust and Node.js bindings
- Comprehensive test corpus covering all language features
- CI pipeline for automated testing and publishing

## Installation

### Neovim (with nvim-treesitter)
```lua
require('nvim-treesitter.configs').setup({
  ensure_installed = { "naso" },
  highlight = { enable = true },
})
```

### Zed
The grammar is automatically available in Zed editor.

### Web / WASM
```javascript
const Parser = require('web-tree-sitter');
const nasoGrammar = await fetch('/tree-sitter-naso.wasm').then(r => r.arrayBuffer());
const nasoLanguage = await Parser.Language.load(nasoGrammar);
const parser = new Parser();
parser.setLanguage(nasoLanguage);
const tree = parser.parse(`
  fn main() {
    let q = qalloc();
    hadamard(q);
    measure(q);
  }
`);
console.log(tree.rootNode.type); // "source_file"
```

## Usage from Rust

Add to your `Cargo.toml`:
```toml
tree-sitter-naso = { git = "https://github.com/naso-lang/tree-sitter-naso" }
```

```rust
use tree_sitter::{Parser, Language};
use tree_sitter_naso::language;

fn main() {
    let mut parser = Parser::new();
    parser.set_language(language()).expect("Failed to load Naso grammar");
    
    let code = r#"
fn quantum_bell() {
    let (q1, q2) = bell_pair();
    hadamard(q1);
    cnot(q1, q2);
    measure(q1);
    measure(q2);
}
"#;
    
    let tree = parser.parse(code, None).unwrap();
    println!("{}", tree.root_node().to_sexp());
}
```

## Usage from Node.js

```bash
npm install tree-sitter-naso
```

```javascript
const naso = require('tree-sitter-naso');
const Parser = require('web-tree-sitter');

async function parseNaso() {
    await Parser.init();
    const parser = new Parser();
    parser.setLanguage(naso);
    
    const code = `
    fn tensor_ops() {
        let A = matmul(x, y);
        let B = transpose(A);
        let C = contract(A, B, i, j);
    }
    `;
    
    const tree = parser.parse(code);
    console.log(tree.rootNode.toString());
}

parseNaso();
```

## Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- [tree-sitter CLI](https://github.com/tree-sitter/tree-sitter/cli) (v0.22+)

### Build Steps
```bash
# Clone repository
git clone https://github.com/naso-lang/tree-sitter-naso.git
cd tree-sitter-naso

# Generate parser files
tree-sitter generate

# Build Rust bindings
cd bindings/rust
cargo build --release

# Build Node.js bindings
cd ../node
npm install
npm run build

# Build WASM parser
cd ..
tree-sitter build-wasm

# Run tests
cargo test --manifest-path=bindings/rust/Cargo.toml
tree-sitter parse --quiet test/corpus/*.naso
```

## Testing

Run the test suite:
```bash
# Rust binding tests
cargo test --manifest-path=bindings/rust/Cargo.toml

# Parser tests with corpus
tree-sitter parse --quiet test/corpus/*.naso

# WASM tests (requires Node.js)
node -e "
const Parser = require('web-tree-sitter');
const fs = require('fs');
async function test() {
  await Parser.init();
  const wasm = fs.readFileSync('./tree-sitter-naso.wasm');
  const lang = await Parser.Language.load(wasm);
  const parser = new Parser();
  parser.setLanguage(lang);
  const tree = parser.parse('fn main() { qalloc(); }');
  console.log('WASM parse OK:', tree.rootNode.type === 'source_file');
}
test().catch(console.error);
"
```

## Grammar Design

The grammar is designed to produce a lossless CST that preserves all syntactic information needed for:
- Syntax highlighting with semantic tokens
- Code navigation and symbol resolution
- Refactoring tools (rename, extract function, etc.)
- Error detection and quick fixes
- Code formatting
- Structural editing

Key design decisions:
1. **QTT Annotations**: Quantity annotations (`[0]`, `[1]`, `[*]`, `[N]`) are parsed as distinct nodes attached to type annotations
2. **Linear Functions**: Function signatures preserve quantity information for linearity checking
3. **Inout Parameters**: Explicitly parsed for MVS borrow-checker integration
4. **Quantum Intrinsics**: Dedicated node types for each quantum operation
5. **Tensor Operations**: Function-call style parsing with special node types for tree-sitter queries
6. **Polyhedral Loops**: Special node types for `forall`, `tile`, `fuse` with schedule annotation support
7. **Error Recovery**: Grammar includes recovery rules for common syntax errors

## Language Coverage

The grammar supports all Naso language features as of Sprint 7:

### Types and Qualifiers
- Primitive types: `bool`, integers, floats, `char`, `str`, `unit`, `never`, `qubit`
- Quantity annotations: `[0]` (erased), `[1]` (linear), `[*]` (heap), `[N]` (bounded)
- Generics: `T`, `T: Trait`, `T: Trait + Other`
- Function types: `fn(T) -> U`
- Tuples: `(T, U)`
- Arrays: `[T; N]`
- References: `&T`, `&mut T`
- Inout: `inout T`

### Declarations
- Functions: `fn foo(x: T) -> T { ... }`
- Structs: `struct Foo { x: T }`
- Enums: `enum Foo { Variant(T) }`
- Type aliases: `type Foo = Bar;`
- Constants: `const X: T = value;`
- Statics: `static X: T = value;`
- Modules: `module foo { ... }`
- Imports: `import { foo } from "bar";`

### Statements
- Let bindings: `let x = expr;`
- Assignments: `x = expr;`
- Returns: `return expr;`
- Control flow: `if`, `else`, `while`, `for`, `match`
- Loops: `forall`, `tile`, `fuse`
- Quantum: `qalloc()`, `qfree(x)`, `hadamard(x)`, `cnot(x,y)`, `measure(x)`, `bell_pair()`, `qft(x)`, `grover_oracle(x, oracle)`
- Tensor: `matmul(x,y)`, `add(x,y)`, `transpose(x)`, `contract(x,y,dim)`, `outer_product(x,y)`, `dot(x,y)`
- Uncomputation: `uncompute x;`
- Defer: `defer { ... }`

### Expressions
- Literals: numbers, strings, booleans, unit `()`
- Variables: `x`
- Function calls: `foo(args...)`
- Method calls: `obj.method(args...)`
- Field access: `obj.field`
- Indexing: `expr[index]`
- Binary ops: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `|`, `^`, `&`, `<<`, `>>`
- Unary ops: `-`, `!`, `~`, `*`, `&`, `mut`
- Casting: `expr as Type`
- Blocks: `{ ... }`
- If expressions: `if cond { then } else { else }`
- Match expressions: `match expr { pat => expr, ... }`
- Lambda expressions: `|args| -> Ret { ... }`
- Array literals: `[expr, expr, ...]`
- Tuple literals: `(expr, expr, ...)`
- Struct literals: `Struct { field: expr, ... }`
- Range expressions: `start..end`, `start..=end`

## Configuration

The grammar supports the following configuration options via Tree-sitter's API:

None currently implemented - all features are always enabled.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes to `grammar.js`
4. Run `tree-sitter generate` to regenerate parser files
5. Add test cases to `test/corpus/` if needed
6. Ensure all tests pass
7. Submit a pull request

### Testing Changes
```bash
# Regenerate parser
tree-sitter generate

# Test with corpus
tree-sitter parse --quiet test/corpus/*.naso

# Run unit tests
cargo test --manifest-path=bindings/rust/Cargo.toml

# Check for conflicts/ambiguities
tree-sitter test
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Tree-sitter team for the excellent parsing infrastructure
- The Naso language design team
- Contributors to the Naso ecosystem