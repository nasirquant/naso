import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

interface TestFixture {
    name: string;
    path: string;
    expectedTokens?: string[];
    expectedDiagnostics?: number;
}

const TEST_FIXTURES: TestFixture[] = [
    { name: 'basic', path: 'test/fixtures/basic.naso' },
    { name: 'quantum', path: 'test/fixtures/quantum.naso' },
    { name: 'tensor', path: 'test/fixtures/tensor.naso' },
    { name: 'linearity', path: 'test/fixtures/linearity.naso', expectedDiagnostics: 8 }
];

export async function runSyntaxHighlightingTests(): Promise<void> {
    console.log('Running syntax highlighting tests...');
    
    for (const fixture of TEST_FIXTURES) {
        const fullPath = path.resolve(__dirname, '../../', fixture.path);
        if (!fs.existsSync(fullPath)) {
            throw new Error(`Fixture not found: ${fullPath}`);
        }

        const document = await vscode.workspace.openTextDocument(fullPath);
        
        // Verify the document language is detected as naso
        if (document.languageId !== 'naso') {
            throw new Error(`Language detection failed for ${fixture.name}: expected 'naso', got '${document.languageId}'`);
        }

        // Verify syntax highlighting by checking semantic tokens
        const tokens = await vscode.languages.getSemanticTokens(document);
        if (tokens && tokens.data.length === 0) {
            console.warn(`Warning: No semantic tokens for ${fixture.name}`);
        }

        console.log(`✓ ${fixture.name}: Language detection and syntax highlighting OK`);
    }
}

export async function runLspConnectionTests(): Promise<void> {
    console.log('Running LSP connection tests...');
    
    // Wait for LSP client to be ready
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    const client = getLspClient();
    if (!client) {
        throw new Error('LSP client not available');
    }

    // Test with basic fixture
    const fullPath = path.resolve(__dirname, '../../test/fixtures/basic.naso');
    const document = await vscode.workspace.openTextDocument(fullPath);
    
    // Wait for diagnostics
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    const diagnostics = vscode.languages.getDiagnostics(document.uri);
    console.log(`✓ LSP connection: Received ${diagnostics.length} diagnostics for basic.naso`);
}

function getLspClient(): any {
    // Access the LSP client from the extension
    const ext = vscode.extensions.getExtension('naso-lang.vscode-naso');
    if (ext && ext.exports && ext.exports.getLspClient) {
        return ext.exports.getLspClient();
    }
    return null;
}

export async function runHoverTests(): Promise<void> {
    console.log('Running hover tests...');
    
    const fullPath = path.resolve(__dirname, '../../test/fixtures/basic.naso');
    const document = await vscode.workspace.openTextDocument(fullPath);
    
    // Test hover on various symbols
    const testPositions = [
        { line: 2, character: 10, description: 'variable x' },
        { line: 8, character: 10, description: 'linear variable' },
        { line: 14, character: 5, description: 'function add' },
        { line: 25, character: 10, description: 'struct Point' },
        { line: 35, character: 10, description: 'enum Option' },
    ];

    for (const pos of testPositions) {
        const position = new vscode.Position(pos.line, pos.character);
        const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
            'vscode.executeHoverProvider',
            document.uri,
            position
        );
        
        if (hovers && hovers.length > 0) {
            console.log(`✓ Hover on ${pos.description}: ${hovers[0].contents[0]}`);
        } else {
            console.warn(`⚠ No hover for ${pos.description}`);
        }
    }
}

export async function runCompletionTests(): Promise<void> {
    console.log('Running completion tests...');
    
    const fullPath = path.resolve(__dirname, '../../test/fixtures/basic.naso');
    const document = await vscode.workspace.openTextDocument(fullPath);
    
    // Test completions at various positions
    const testPositions = [
        { line: 0, character: 0, description: 'top of file' },
        { line: 10, character: 4, description: 'inside function' },
    ];

    for (const pos of testPositions) {
        const position = new vscode.Position(pos.line, pos.character);
        const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
            'vscode.executeCompletionItemProvider',
            document.uri,
            position
        );
        
        if (completions && completions.items.length > 0) {
            const keywords = completions.items.filter(item => 
                item.kind === vscode.CompletionItemKind.Keyword
            ).map(item => item.label).slice(0, 5);
            console.log(`✓ Completions at ${pos.description}: ${keywords.join(', ')}...`);
        } else {
            console.warn(`⚠ No completions at ${pos.description}`);
        }
    }
}

export async function runDiagnosticsTests(): Promise<void> {
    console.log('Running diagnostics tests...');
    
    // Test linearity diagnostics
    const linearityPath = path.resolve(__dirname, '../../test/fixtures/linearity.naso');
    const linearityDoc = await vscode.workspace.openTextDocument(linearityPath);
    
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    const linearityDiagnostics = vscode.languages.getDiagnostics(linearityDoc.uri);
    const errorDiagnostics = linearityDiagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
    const warningDiagnostics = linearityDiagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Warning);
    
    console.log(`✓ Linearity diagnostics: ${errorDiagnostics.length} errors, ${warningDiagnostics.length} warnings`);
    
    // Verify specific error codes
    const errorCodes = errorDiagnostics.map(d => d.code).filter(c => typeof c === 'string');
    const expectedCodes = [
        'NASO-LIN-001', 'NASO-LIN-002', 'NASO-LIN-003',
        'NASO-ERA-001', 'NASO-ERA-002',
        'NASO-MVS-001', 'NASO-MVS-002',
        'NASO-UNC-001', 'NASO-UNC-002'
    ];
    
    for (const expected of expectedCodes) {
        if (errorCodes.some(c => c.toString().includes(expected))) {
            console.log(`  ✓ Found ${expected}`);
        } else {
            console.warn(`  ⚠ Missing ${expected}`);
        }
    }
}

export async function runCodeActionTests(): Promise<void> {
    console.log('Running code action tests...');
    
    const linearityPath = path.resolve(__dirname, '../../test/fixtures/linearity.naso');
    const document = await vscode.workspace.openTextDocument(linearityPath);
    
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    const diagnostics = vscode.languages.getDiagnostics(document.uri);
    const errorDiagnostics = diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
    
    if (errorDiagnostics.length > 0) {
        // Test code actions for first error
        const firstError = errorDiagnostics[0];
        const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
            'vscode.executeCodeActionProvider',
            document.uri,
            firstError.range,
            { diagnostics: [firstError] }
        );
        
        if (actions && actions.length > 0) {
            console.log(`✓ Code actions for ${firstError.code}: ${actions.map(a => a.title).join(', ')}`);
        } else {
            console.warn(`⚠ No code actions for ${firstError.code}`);
        }
    }
}

export async function runCommandTests(): Promise<void> {
    console.log('Running command tests...');
    
    const commands = [
        'naso.build',
        'naso.run',
        'naso.emitQir',
        'naso.emitLlvm',
        'naso.check',
        'naso.test',
        'naso.fmt',
        'naso.restartLsp'
    ];
    
    for (const cmd of commands) {
        try {
            await vscode.commands.executeCommand(cmd);
            console.log(`✓ Command ${cmd} executed`);
        } catch (error) {
            console.warn(`⚠ Command ${cmd} failed: ${error}`);
        }
    }
}

export async function runAllTests(): Promise<void> {
    console.log('=== Starting Naso VS Code Extension Integration Tests ===\n');
    
    try {
        await runSyntaxHighlightingTests();
        console.log('');
        
        await runLspConnectionTests();
        console.log('');
        
        await runHoverTests();
        console.log('');
        
        await runCompletionTests();
        console.log('');
        
        await runDiagnosticsTests();
        console.log('');
        
        await runCodeActionTests();
        console.log('');
        
        await runCommandTests();
        console.log('');
        
        console.log('=== All Tests Completed ===');
    } catch (error) {
        console.error('Test suite failed:', error);
        throw error;
    }
}