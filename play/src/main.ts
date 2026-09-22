// Naso Playground - Main Application Entry Point
// Initializes Monaco Editor, loads WASM compiler, and handles compilation

import * as monaco from 'monaco-editor';
import { registerNasoLanguage, NASO_LANGUAGE_ID, DEFAULT_PROGRAM } from './editor/naso-lang';
import './wasm-wrapper.ts'; // Initialize WASM module

// WASM module types (re-exported from wasm-wrapper)
import type { Diagnostic, CompileResult, ParseResult } from './vite-env';
import type { WasmCompileResult, WasmDiagnostic, WasmParseResult, WasmToken } from './vite-env';
import * as wasmModuleExports from './wasm-wrapper.ts';

// Global state
interface WasmModule {
  compile_naso_wasm: (source: string) => WasmCompileResult;
  parse_naso: (source: string) => WasmParseResult;
  tokenize_naso: (source: string) => WasmToken[];
  default_naso_program: () => string;
}
let wasmModule: WasmModule | null = null;
let wasmReadyResolve: (value: WasmModule) => void;
const wasmReadyPromise = new Promise<WasmModule>((resolve) => {
  wasmReadyResolve = resolve;
});
let editor: monaco.editor.IStandaloneCodeEditor | null = null;
let compileDebounceTimer: ReturnType<typeof setTimeout> | null = null;
const DEBOUNCE_MS = 300;

// DOM elements
const loadingOverlay = document.getElementById('loadingOverlay')!;
const editorContainer = document.getElementById('editor')!;
const btnCompile = document.getElementById('btnCompile')! as HTMLButtonElement;
const btnFormat = document.getElementById('btnFormat')! as HTMLButtonElement;
const wasmStatusDot = document.getElementById('wasmStatus')!;
const wasmStatusText = document.getElementById('wasmStatusText')!;
const cursorLineEl = document.getElementById('cursorLine')!;
const cursorColEl = document.getElementById('cursorCol')!;
const editorStatusEl = document.getElementById('editorStatus')!;

// Inspector panels
const diagnosticsPanel = document.getElementById('panelDiagnostics')!;
const astPanel = document.getElementById('panelAST')!;
const inversePanel = document.getElementById('panelInverse')!;
const diagnosticsEmpty = document.getElementById('diagnosticsEmpty')!;
const astEmpty = document.getElementById('astEmpty')!;
const inverseEmpty = document.getElementById('inverseEmpty')!;
const diagnosticsList = document.getElementById('diagnosticsList')!;
const astTree = document.getElementById('astTree')!;
const inverseTree = document.getElementById('inverseTree')!;

// Tab switching
document.querySelectorAll('.inspector-tab').forEach(tab => {
  tab.addEventListener('click', () => {
    const panelId = tab.getAttribute('data-panel');
    if (!panelId) return;
    
    document.querySelectorAll('.inspector-tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.inspector-panel').forEach(p => p.classList.remove('active'));
    
    tab.classList.add('active');
    document.getElementById(`panel${panelId.charAt(0).toUpperCase() + panelId.slice(1)}`)?.classList.add('active');
  });
});

// Initialize Monaco Editor
async function initializeEditor() {
  // Wait for Monaco to be ready
  await new Promise<void>(resolve => {
    if (typeof monaco !== 'undefined') {
      resolve();
    } else {
      (window as any).onMonacoLoad = resolve;
    }
  });

  // Register Naso language
  registerNasoLanguage();

  // Create editor
  editor = monaco.editor.create(editorContainer, {
    value: DEFAULT_PROGRAM,
    language: NASO_LANGUAGE_ID,
    theme: 'naso-dark',
    automaticLayout: true,
    minimap: { enabled: false },
    fontSize: 14,
    lineNumbers: 'on',
    renderLineHighlight: 'all',
    scrollBeyondLastLine: false,
    folding: true,
    bracketPairColorization: { enabled: true },
    guides: {
      bracketPairs: true,
      indentation: true,
    },
    tabSize: 2,
    insertSpaces: true,
    detectIndentation: false,
    wordWrap: 'on',
    smoothScrolling: true,
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
  });

  // Set up cursor position tracking
  editor.onDidChangeCursorPosition(e => {
    cursorLineEl.textContent = e.position.lineNumber.toString();
    cursorColEl.textContent = e.position.column.toString();
  });

  // Debounced compilation on change
  editor.onDidChangeModelContent(() => {
    if (compileDebounceTimer) {
      clearTimeout(compileDebounceTimer);
    }
    compileDebounceTimer = setTimeout(() => {
      compile();
    }, DEBOUNCE_MS);
    
    editorStatusEl.textContent = 'Modified';
  });

  // Format button
  btnFormat.addEventListener('click', () => {
    editor?.getAction('editor.action.formatDocument')?.run();
  });

  // Compile button (manual trigger)
  btnCompile.addEventListener('click', () => {
    compile();
  });

  // Keyboard shortcut: Ctrl+Enter to compile
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
    compile();
  });

  // Keyboard shortcut: Alt+F to format
  editor.addCommand(monaco.KeyMod.Alt | monaco.KeyCode.KeyF, () => {
    editor?.getAction('editor.action.formatDocument')?.run();
  });

  // Initial compile
  await compile();
}

// Initialize WASM module
async function initializeWasm() {
  try {
    wasmStatusText.textContent = 'Initializing...';
    wasmStatusDot.classList.add('compiling');
    
    // The wasm-wrapper.ts has already initialized the wasm module
    // Just get the wasm module exports
    wasmModule = wasmModuleExports as unknown as WasmModule;
    wasmReadyResolve(wasmModule);
    
    wasmStatusDot.classList.remove('compiling');
    wasmStatusDot.classList.add('connected');
    wasmStatusText.textContent = 'Ready';
    
    // Hide loading overlay
    loadingOverlay.classList.add('hidden');
    
    // Initialize editor after WASM is ready
    await initializeEditor();
  } catch (error) {
    console.error('Failed to initialize WASM:', error);
    wasmStatusDot.classList.remove('compiling');
    wasmStatusText.textContent = 'Error';
    loadingOverlay.classList.add('hidden');
    
    // Resolve with null so compile() knows WASM failed
    wasmReadyResolve(null as any);
    
    // Still try to initialize editor (without WASM)
    await initializeEditor();
  }
}

// Compile current editor content
async function compile() {
  if (!editor) {
    return;
  }

  // Wait for WASM to be ready (or fail)
  const module = await wasmReadyPromise;
  if (!module) {
    showDiagnostics([{
      severity: 'error',
      message: 'WASM compiler failed to load',
      line: 1,
      column: 1,
      end_line: 1,
      end_column: 10,
      code: 'WASM_LOAD_FAILED',
    }]);
    return;
  }
  wasmModule = module;

  const source = editor.getValue();
  if (!source.trim()) {
    clearDiagnostics();
    return;
  }

  editorStatusEl.textContent = 'Compiling...';
  wasmStatusDot.classList.add('compiling');
  wasmStatusText.textContent = 'Compiling...';
  btnCompile.disabled = true;

  try {
    // Call WASM compile function
    const result = wasmModule.compile_naso_wasm(source);
    
    // Convert wasm diagnostics to local diagnostics
    const localDiagnostics: Diagnostic[] = result.diagnostics.map(convertWasmDiagnostic);
    
    // Update UI with results
    showDiagnostics(localDiagnostics);
    showAST(result.ast_json);
    showInverseDAG(result.inverse_dag_json);
    
    // Add error decorations to editor
    addErrorDecorations(localDiagnostics);
    
    editorStatusEl.textContent = result.success ? 'OK' : 'Errors';
    wasmStatusDot.classList.remove('compiling');
    wasmStatusDot.classList.add('connected');
    wasmStatusText.textContent = 'Ready';
  } catch (error) {
    console.error('Compilation error:', error);
    showDiagnostics([{
      severity: 'error',
      message: `Compilation failed: ${error}`,
      line: 1,
      column: 1,
      end_line: 1,
      end_column: 10,
      code: 'COMPILATION_ERROR',
    }]);
    editorStatusEl.textContent = 'Failed';
    wasmStatusDot.classList.remove('compiling');
    wasmStatusText.textContent = 'Error';
  } finally {
    btnCompile.disabled = false;
  }
}

// Convert wasm diagnostic to local diagnostic
function convertWasmDiagnostic(d: WasmDiagnostic): Diagnostic {
  // wasm-pack classes may have getters (severity) or methods (severity())
  // Try calling as method first, then fall back to property access
  const getVal = (obj: WasmDiagnostic, key: string): unknown => {
    const val = (obj as Record<string, unknown>)[key];
    return typeof val === 'function' ? val() : val;
  };
  const local: Diagnostic = {
    severity: getVal(d, 'severity') as Diagnostic['severity'],
    message: getVal(d, 'message') as string,
    line: getVal(d, 'line') as number,
    column: getVal(d, 'column') as number,
    end_line: getVal(d, 'end_line') as number,
    end_column: getVal(d, 'end_column') as number,
    code: getVal(d, 'code') as string,
  };
  // NOTE: Skip free() - wasm objects are GC'd by JS runtime.
  // Calling free() can cause "__destroy_into_raw" errors if the object
  // is already moved or doesn't implement the expected destructor pattern.
  return local;
}

// Show diagnostics in inspector
function showDiagnostics(diagnostics: Diagnostic[]) {
  if (diagnostics.length === 0) {
    diagnosticsEmpty.style.display = 'flex';
    diagnosticsList.innerHTML = '';
    return;
  }
  
  diagnosticsEmpty.style.display = 'none';
  diagnosticsList.innerHTML = diagnostics.map(d => renderDiagnostic(d)).join('');
}

// Render a single diagnostic
function renderDiagnostic(d: Diagnostic): string {
  const severityClass = d.severity.toLowerCase();
  const codeHtml = d.code ? `<span class="diagnostic-code">${d.code}</span>` : '';
  
  return `
    <div class="diagnostic ${severityClass} severity-${severityClass}">
      <div class="diagnostic-header">
        <span class="diagnostic-severity">${d.severity}</span>
        <span class="diagnostic-location">${d.line}:${d.column} - ${d.end_line}:${d.end_column}</span>
        ${codeHtml}
      </div>
      <div class="diagnostic-message">${escapeHtml(d.message)}</div>
    </div>
  `;
}

// Show AST in inspector
function showAST(astJson: string | null | undefined) {
  if (!astJson) {
    astEmpty.style.display = 'flex';
    astTree.textContent = '';
    return;
  }
  
  astEmpty.style.display = 'none';
  try {
    const parsed = JSON.parse(astJson);
    astTree.textContent = JSON.stringify(parsed, null, 2);
    highlightJson(astTree);
  } catch {
    astTree.textContent = astJson;
  }
}

// Show Inverse DAG in inspector
function showInverseDAG(dagJson: string | null | undefined) {
  if (!dagJson) {
    inverseEmpty.style.display = 'flex';
    inverseTree.textContent = '';
    return;
  }
  
  inverseEmpty.style.display = 'none';
  try {
    const parsed = JSON.parse(dagJson);
    inverseTree.textContent = JSON.stringify(parsed, null, 2);
    highlightJson(inverseTree);
  } catch {
    inverseTree.textContent = dagJson;
  }
}

// Simple JSON syntax highlighting for <pre> elements
function highlightJson(element: HTMLElement) {
  const text = element.textContent || '';
  try {
    const parsed = JSON.parse(text);
    element.innerHTML = syntaxHighlightJson(parsed);
  } catch {
    // Keep as plain text if not valid JSON
  }
}

function syntaxHighlightJson(obj: any, indent = 0): string {
  const spaces = '  '.repeat(indent);
  const nextSpaces = '  '.repeat(indent + 1);
  
  if (obj === null) {
    return '<span class="null">null</span>';
  }
  
  switch (typeof obj) {
    case 'string':
      return `<span class="string">"${escapeHtml(obj)}"</span>`;
    case 'number':
      return `<span class="number">${obj}</span>`;
    case 'boolean':
      return `<span class="boolean">${obj}</span>`;
    case 'object':
      if (Array.isArray(obj)) {
        if (obj.length === 0) return '[]';
        const items = obj.map(item => 
          `${nextSpaces}${syntaxHighlightJson(item, indent + 1)}`
        ).join(',\n');
        return `[\n${items}\n${spaces}]`;
      } else {
        const keys = Object.keys(obj);
        if (keys.length === 0) return '{}';
        const items = keys.map(key => 
          `${nextSpaces}<span class="key">"${escapeHtml(key)}"</span>: ${syntaxHighlightJson(obj[key], indent + 1)}`
        ).join(',\n');
        return `{\n${items}\n${spaces}}`;
      }
    default:
      return escapeHtml(String(obj));
  }
}

// Add error decorations to Monaco editor
function addErrorDecorations(diagnostics: Diagnostic[]) {
  if (!editor) return;
  
  const decorations = diagnostics
    .filter(d => d.severity === 'error')
    .map(d => ({
      range: new monaco.Range(d.line, d.column, d.end_line, d.end_column),
      options: {
        className: 'squiggly-error',
        hoverMessage: { value: d.message },
        stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
      },
    }));
  
  editor.deltaDecorations([], decorations);
}

// Clear all diagnostics
function clearDiagnostics() {
  diagnosticsEmpty.style.display = 'flex';
  diagnosticsList.innerHTML = '';
  astEmpty.style.display = 'flex';
  astTree.textContent = '';
  inverseEmpty.style.display = 'flex';
  inverseTree.textContent = '';
  editor?.deltaDecorations([], []);
}

// HTML escape utility
function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Add custom CSS for error squiggles
const style = document.createElement('style');
style.textContent = `
  .squiggly-error {
    border-bottom: 2px wavy var(--accent-red);
  }
  
  .monaco-editor .squiggly-error {
    border-bottom: 2px wavy #f85149;
  }
`;
document.head.appendChild(style);

// Start the application
initializeWasm();

// Export for debugging
(window as any).__NASO_PLAYGROUND__ = {
  getEditor: () => editor,
  getWasmModule: () => wasmModule,
  compile: compile,
};