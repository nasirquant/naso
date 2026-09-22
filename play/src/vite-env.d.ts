/// <reference types="vite/client" />

declare module '*.wasm' {
  const content: string;
  export default content;
}

declare module '../pkg/nasoc_wasm.js' {
  export function init(wasmUrl?: string | URL): Promise<void>;
  export function compile_naso_wasm(source: string): WasmCompileResult;
  export function parse_naso(source: string): WasmParseResult;
  export function tokenize_naso(source: string): WasmToken[];
  export function default_naso_program(): string;

  export interface WasmCompileResult {
    success: boolean;
    ast_json: string | null;
    inverse_dag_json: string | null;
    diagnostics: WasmDiagnostic[];
  }

  export interface WasmDiagnostic {
    severity(): string;
    message(): string;
    line(): number;
    column(): number;
    end_line(): number;
    end_column(): number;
    code(): string | null;
    free(): void;
  }

  export interface WasmParseResult {
    success: boolean;
    ast_json: string | null;
    error: string | null;
  }

  export interface WasmToken {
    kind(): string;
    text(): string;
    start(): number;
    end(): number;
    line(): number;
    column(): number;
    free(): void;
  }
}

// Local types for the application
export interface Diagnostic {
  severity: string;
  message: string;
  line: number;
  column: number;
  end_line: number;
  end_column: number;
  code: string | null;
}

export interface CompileResult {
  success: boolean;
  ast_json: string | null;
  inverse_dag_json: string | null;
  diagnostics: Diagnostic[];
}

export interface ParseResult {
  success: boolean;
  ast_json: string | null;
  error: string | null;
}

export interface WasmToken {
  kind: string;
  text: string;
  start: number;
  end: number;
  line: number;
  column: number;
}

// Re-export wasm types for use in main.ts
export type WasmCompileResult = WasmCompileResultInternal;
export type WasmDiagnostic = WasmDiagnosticInternal;
export type WasmParseResult = WasmParseResultInternal;