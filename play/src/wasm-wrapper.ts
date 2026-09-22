// WASM module wrapper for Vite
// This wrapper imports the wasm file using Vite's ?url import
// and initializes the wasm-pack module with the correct wasm URL.

import wasmUrl from '../pkg/nasoc_wasm_bg.wasm?url';
import * as wasmModule from '../pkg/nasoc_wasm.js';

// Initialize the wasm-pack module with the Vite-resolved wasm URL
const initFn = wasmModule.default || wasmModule.initSync;
if (typeof initFn === 'function') {
  await initFn({ module_or_path: wasmUrl });
}

// Re-export all wasm-pack exports
export * from '../pkg/nasoc_wasm.js';