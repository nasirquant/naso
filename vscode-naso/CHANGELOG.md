# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-12

### Added
- **VS Code Extension Scaffold**: Complete extension structure for Naso language support
- **Syntax Highlighting**: TextMate grammar (`naso.tmLanguage.json`) covering all Naso constructs:
  - QTT quantity annotations (`[0]`, `[1]`, `[*]`, `[N]`)
  - Linear function signatures and `inout` parameters
  - Quantum intrinsics (`qalloc`, `hadamard`, `cnot`, `measure`, etc.)
  - Tensor operations (`matmul`, `contract`, `transpose`, `einsum`, etc.)
  - Polyhedral loop constructs (`forall`, `tile`, `fuse`, `#[schedule(...)]`)
  - Module system, structs, enums, type aliases
- **Language Configuration**: Brackets, auto-closing pairs, comments, folding markers
- **Code Snippets**: 30+ snippets for common Naso patterns:
  - Function templates (linear, proof, inout, quantum)
  - Struct/enum/type alias definitions
  - Quantum circuits (Bell pair, QFT, Grover)
  - Tensor operations (matmul, contraction, decompositions)
  - Polyhedral loops (forall, tiled, fused)
- **LSP Client Integration** (`lsp-client.ts`):
  - LanguageClient connection to `naso-lsp` via stdio
  - Automatic restart with exponential backoff on crash
  - Status bar indicator for connection state
  - Configurable server path and trace level
- **Build Commands** (`commands.ts`):
  - `Naso: Build` (with target selection: llvm/qir/jit)
  - `Naso: Run`
  - `Naso: Emit QIR`
  - `Naso: Emit LLVM IR`
  - Additional: check, test, fmt, clippy, doc
- **Extension Entry Point** (`extension.ts`):
  - Activation on `onLanguage:naso`
  - Configuration change handling
  - Command registration
- **Integration Test Suite** (`test/suite/integration.test.ts`):
  - Syntax highlighting verification
  - LSP connection testing
  - Hover provider testing
  - Completion provider testing
  - Diagnostics validation (all 9 QTT error codes)
  - Code action (quick fix) testing
  - Command execution testing
- **Test Fixtures** (`test/fixtures/`):
  - `basic.naso`: Core language features
  - `quantum.naso`: Quantum circuits and intrinsics
  - `tensor.naso`: Tensor operations and polyhedral loops
  - `linearity.naso`: QTT linearity/MVS/error test cases
- **CI/CD Pipeline** (`.github/workflows/ci.yml`):
  - Extension build, lint, test
  - naso-lsp build and test
  - tree-sitter-naso parser build and test
  - VSIX packaging and marketplace publishing
- **Development Configuration**:
  - TypeScript config with strict mode
  - VS Code launch configurations (extension + tests)
  - ESLint + TypeScript ESLint config

### Configuration
- `naso.lsp.enable`: Enable/disable LSP (default: true)
- `naso.lsp.serverPath`: Path to naso-lsp binary (default: "naso-lsp")
- `naso.lsp.trace.server`: LSP trace level (off/messages/verbose)
- `naso.build.target`: Default build target (llvm/qir/jit)
- `naso.format.enable`: Format on save (default: true)

### Diagnostic Codes Supported
| Code | Category | Description |
|------|----------|-------------|
| NASO-LIN-001 | Linearity | Unused linear value |
| NASO-LIN-002 | Linearity | Double-use of linear value |
| NASO-LIN-003 | Linearity | Implicit drop of linear value |
| NASO-ERA-001 | Erasure | Retained `[0]` value at runtime |
| NASO-ERA-002 | Erasure | Non-erased proof |
| NASO-MVS-001 | MVS | Inout aliasing conflict |
| NASO-MVS-002 | MVS | Inout escape |
| NASO-UNC-001 | Uncomputation | Uncomputation failed |
| NASO-UNC-002 | Uncomputation | Non-invertible temporary |

### Dependencies
- `vscode-languageclient` ^9.0.0
- Dev: `@types/vscode` ^1.85.0, `vscode-test` ^1.6.0, `typescript` ^5.3.0

---

## [Unreleased]

### Planned
- Tree-sitter based semantic highlighting (replacing TextMate)
- Inlay hints for quantity annotations
- Debug adapter protocol (DAP) integration
- Interactive quantum circuit visualization
- Tensor shape inference display
- Polyhedral schedule visualization