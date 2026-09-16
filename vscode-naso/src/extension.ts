import * as vscode from 'vscode';
import { NasoLspClient } from './lsp-client';
import { NasoCommands, registerCommands } from './commands';

let lspClient: NasoLspClient;
let nasoCommands: NasoCommands;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    console.log('Naso extension activating...');

    // Create output channel for logging
    const outputChannel = vscode.window.createOutputChannel('Naso');
    context.subscriptions.push(outputChannel);

    // Initialize commands
    nasoCommands = new NasoCommands(outputChannel);
    registerCommands(context, nasoCommands);

    // Initialize LSP client
    lspClient = new NasoLspClient(context);
    
    // Start LSP client
    await lspClient.start();

    // Register configuration change listener
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(event => {
            if (event.affectsConfiguration('naso.lsp') || event.affectsConfiguration('naso.build')) {
                lspClient.updateConfig();
            }
        })
    );

    // Register restart command
    context.subscriptions.push(
        vscode.commands.registerCommand('naso.restartLsp', async () => {
            await lspClient.restart();
        })
    );

    // Show activation message
    vscode.window.showInformationMessage('Naso language support activated!', 'Show Output').then(selection => {
        if (selection === 'Show Output') {
            outputChannel.show();
        }
    });

    console.log('Naso extension activated successfully');
}

export function deactivate(): Promise<void> | undefined {
    console.log('Naso extension deactivating...');
    
    if (lspClient) {
        return lspClient.dispose();
    }
    
    if (nasoCommands) {
        nasoCommands.dispose();
    }
    
    console.log('Naso extension deactivated');
    return undefined;
}