//! Hover Handler - Provides type information on hover

use crate::NasoLanguageServer;
use tower_lsp::lsp_types::*;

pub async fn handle_hover(
    server: &NasoLanguageServer,
    params: HoverParams,
) -> Result<Option<Hover>, tower_lsp::jsonrpc::Error> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    tracing::debug!("Hover request: {} at {:?}", uri, position);

    // Get hover info from compiler bridge
    if let Some(hover_info) = server.compiler_bridge.get_hover(&uri, position).await {
        let mut contents = Vec::new();

        // Type information
        contents.push(MarkedString::String(format!(
            "**Type:** `{}`",
            hover_info.type_info
        )));

        // Quantity annotation
        if let Some(qty) = hover_info.quantity {
            contents.push(MarkedString::String(format!("**Quantity:** `{}`", qty)));
        }

        // Documentation
        if let Some(doc) = hover_info.doc_comment {
            contents.push(MarkedString::String(format!("---\n{}", doc)));
        }

        return Ok(Some(Hover {
            contents: HoverContents::Array(contents),
            range: Some(hover_info.range),
        }));
    }

    // Fallback: check if there's a word at position and provide generic info
    #[allow(clippy::collapsible_if)]
    if let Some(document) = server.document_store.get(&uri) {
        if let Some(word) = extract_word_at(&document.content, position) {
            if let Some(generic_hover) = get_generic_hover(&word) {
                return Ok(Some(Hover {
                    contents: HoverContents::Array(vec![MarkedString::String(generic_hover)]),
                    range: None,
                }));
            }
        }
    }

    Ok(None)
}

fn extract_word_at(content: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let line = lines.get(position.line as usize)?;

    let char_idx = position.character as usize;
    if char_idx >= line.len() {
        return None;
    }

    let mut start = char_idx;
    let mut end = char_idx;

    // Expand to word boundaries (alphanumeric and underscore)
    while start > 0 {
        let ch = line.chars().nth(start - 1)?;
        if ch.is_alphanumeric() || ch == '_' {
            start -= 1;
        } else {
            break;
        }
    }

    while end < line.len() {
        let ch = line.chars().nth(end)?;
        if ch.is_alphanumeric() || ch == '_' {
            end += 1;
        } else {
            break;
        }
    }

    if start < end {
        Some(line[start..end].to_string())
    } else {
        None
    }
}

fn get_generic_hover(word: &str) -> Option<String> {
    // QTT Quantity annotations
    match word {
        "[0]" => Some("**Quantity:** `[0]` (Proof/Erased)\n\nValues marked `[0]` exist only for type-checking and proofs. They are **completely erased at runtime** and generate zero code. Use for: phantom types, proof witnesses, compile-time computations.".to_string()),
        "[1]" => Some("**Quantity:** `[1]` (Linear)\n\nValues marked `[1]` must be **consumed exactly once**. The compiler enforces linear usage: no implicit drops, no double-use, no aliasing. Automatic uncomputation is triggered when the value goes out of scope. Use for: owned resources, quantum states, unique pointers.".to_string()),
        "[*]" => Some("**Quantity:** `[*]` (Unrestricted)\n\nValues marked `[*]` have **unrestricted usage** - they can be copied, dropped, aliased freely. Standard Rust-like semantics. Use for: classical data, shared references, Copy types.".to_string()),
        _ if word.starts_with('[') && word.ends_with(']') && word[1..word.len()-1].parse::<usize>().is_ok() => {
            let n = word[1..word.len()-1].parse::<usize>().ok()?;
            Some(format!("**Quantity:** `[{n}]` (Bounded)\n\nValues marked `[{n}]` have **bounded quantity** - they can be used up to {n} times. The compiler tracks usage count statically. Use for: fixed-size arrays, bounded buffers, limited resources."))
        }
        // Quantum intrinsics
        "qalloc" => Some("**Function:** `qalloc() -> [1] Qubit`

**Quantity:** `[1]` (Linear)

Allocate a new qubit initialized to `|0⟩`. Returns a linear qubit that must be consumed exactly once. Automatic uncomputation will be inserted at scope exit.".to_string()),
        "hadamard" => Some("**Function:** `hadamard(q: [1] Qubit) -> [1] Qubit`

**Quantity:** `[1]` (Linear)

Apply the Hadamard gate: `H = 1/√2 [[1, 1], [1, -1]]`. Consumes the input qubit linearly and returns the transformed qubit.".to_string()),
        "cnot" => Some("**Function:** `cnot(ctrl: [1] Qubit, target: [1] Qubit) -> ([1] Qubit, [1] Qubit)`\n\n**Quantity:** `[1]` (Linear)\n\nApply CNOT (controlled-X) gate. Both control and target qubits are consumed linearly and returned.".to_string()),
        "measure" => Some("**Function:** `measure(q: [1] Qubit) -> Bool`\n\n**Quantity:** `[1]` (Linear)\n\nMeasure qubit in computational basis (`|0⟩`/`|1⟩`). Collapses the quantum state and returns a classical `Bool`. Consumes the qubit.".to_string()),
        "qfree" => Some("**Function:** `qfree(q: [1] Qubit)`\n\n**Quantity:** `[1]` (Linear)\n\nExplicitly free a qubit, triggering uncomputation. **Use sparingly** - automatic uncomputation at scope exit is preferred. Only use when you need to free a qubit earlier than its lexical scope.".to_string()),
        // Tensor operations
        "matmul" => Some("**Function:** `matmul<T, const M: usize, const K: usize, const L: usize>(a: Tensor<[N]; [M, K]>, b: Tensor<[N]; [K, L]>) -> Tensor<[N]; [M, L]>`\n\n**Quantity:** `[*]` (Unrestricted)\n\nMatrix multiplication with polyhedral optimization. Supports tiling, fusion, and hardware-specific codegen (CPU/GPU/TPU).".to_string()),
        "contract" => Some("**Function:** `contract<T, Dims1, Dims2, DimsOut>(a: Tensor<[N]; Dims1>, b: Tensor<[N]; Dims2>) -> Tensor<[N]; DimsOut>`\n\n**Quantity:** `[*]` (Unrestricted)\n\nTensor contraction over matching dimensions. Einstein summation convention. Optimized via polyhedral IR.".to_string()),
        "transpose" => Some("**Function:** `transpose<T, const M: usize, const N: usize>(t: Tensor<[K]; [M, N]>) -> Tensor<[K]; [N, M]>`\n\n**Quantity:** `[*]` (Unrestricted)\n\nTranspose a 2D tensor. Zero-cost for compatible memory layouts.".to_string()),
        // MVS keywords
        "inout" => Some("**Keyword:** `inout` (Mutable Value Semantics)\n\nDeclares a parameter as a **mutable value reference**. Unlike Rust's `&mut`, `inout` provides:\n- No aliasing guarantees required (compiler enforces)\n- No lifetime annotations\n- Local mutation within function scope\n- Value semantics: the original is updated on return\n\nExample: `fn swap(inout a: T, inout b: T)`".to_string()),
        _ => None,
    }
}
