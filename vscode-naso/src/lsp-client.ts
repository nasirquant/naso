import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from 'vscode-languageclient/node';

export interface LspClientConfig {
    serverPath: string;
    enable: boolean;
    trace: 'off' | 'messages' | 'verbose';
}

export class NasoLspClient {
    private client: LanguageClient | undefined;
    private config: LspClientConfig;
    private restartAttempts = 0;
    private readonly maxRestartAttempts = 5;
    private readonly restartDelayBase = 1000; // ms
    private outputChannel: vscode.OutputChannel;
    private statusBarItem: vscode.StatusBarItem;

    constructor(context: vscode.ExtensionContext) {
        this.config = this.loadConfig();
        this.outputChannel = vscode.window.createOutputChannel('Naso Language Server');
        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
        this.statusBarItem.command = 'naso.restartLsp';
        this.updateStatusBar('Starting...');
        this.statusBarItem.show();
        context.subscriptions.push(this.outputChannel, this.statusBarItem);
    }

    private loadConfig(): LspClientConfig {
        const config = vscode.workspace.getConfiguration('naso.lsp');
        return {
            serverPath: config.get<string>('serverPath', 'naso-lsp'),
            enable: config.get<boolean>('enable', true),
            trace: config.get<'off' | 'messages' | 'verbose'>('trace.server', 'off')
        };
    }

    public async start(): Promise<void> {
        if (!this.config.enable) {
            this.updateStatusBar('Disabled');
            this.log('LSP client disabled by configuration');
            return;
        }

        this.updateStatusBar('Connecting...');
        
        const serverOptions: ServerOptions = {
            command: this.config.serverPath,
            args: [],
            transport: TransportKind.stdio,
            options: {
                env: { ...process.env, RUST_LOG: this.config.trace === 'verbose' ? 'debug' : 'info' }
            }
        };

        const clientOptions: LanguageClientOptions = {
            documentSelector: [
                { scheme: 'file', language: 'naso' }
            ],
            synchronize: {
                fileEvents: vscode.workspace.createFileSystemWatcher('**/*.naso')
            },
            outputChannel: this.outputChannel,
            traceOutputChannel: this.outputChannel,
            initializationOptions: {},
            middleware: {
                handleDiagnostics: (uri, diagnostics, next) => {
                    // Filter or enhance diagnostics if needed
                    next(uri, diagnostics);
                }
            }
        };

        this.client = new LanguageClient('naso-lsp', 'Naso Language Server', serverOptions, clientOptions);

        this.client.onDidChangeState((event) => {
            this.log(`LSP state changed: ${LanguageClient.State[event.oldState]} -> ${LanguageClient.State[event.newState]}`);
            this.updateStatusBarFromState(event.newState);
        });

        this.client.onDidStop(() => {
            this.log('LSP server stopped');
            if (this.config.enable) {
                this.scheduleRestart();
            }
        });

        try {
            await this.client.start();
            this.restartAttempts = 0;
            this.log('LSP client started successfully');
            this.updateStatusBar('Connected');
        } catch (error) {
            this.log(`Failed to start LSP client: ${error}`);
            this.updateStatusBar('Failed');
            this.scheduleRestart();
        }
    }

    public async stop(): Promise<void> {
        if (this.client) {
            this.log('Stopping LSP client...');
            await this.client.stop();
            this.client = undefined;
            this.updateStatusBar('Stopped');
            this.log('LSP client stopped');
        }
    }

    public async restart(): Promise<void> {
        this.log('Restarting LSP client...');
        await this.stop();
        this.restartAttempts = 0;
        await this.start();
    }

    private scheduleRestart(): void {
        if (this.restartAttempts >= this.maxRestartAttempts) {
            this.log(`Max restart attempts (${this.maxRestartAttempts}) reached. Giving up.`);
            this.updateStatusBar('Failed (max retries)');
            vscode.window.showErrorMessage(
                'Naso Language Server failed to start after multiple attempts. Check the output channel for details.',
                'Show Output'
            ).then(selection => {
                if (selection === 'Show Output') {
                    this.outputChannel.show();
                }
            });
            return;
        }

        const delay = this.restartDelayBase * Math.pow(2, this.restartAttempts);
        this.restartAttempts++;
        this.log(`Scheduling restart attempt ${this.restartAttempts}/${this.maxRestartAttempts} in ${delay}ms`);
        this.updateStatusBar(`Reconnecting (${this.restartAttempts}/${this.maxRestartAttempts})...`);

        setTimeout(() => {
            this.start().catch(err => this.log(`Restart failed: ${err}`));
        }, delay);
    }

    private updateStatusBarFromState(state: LanguageClient.State): void {
        switch (state) {
            case LanguageClient.State.Running:
                this.updateStatusBar('Connected');
                break;
            case LanguageClient.State.Starting:
                this.updateStatusBar('Connecting...');
                break;
            case LanguageClient.State.Stopped:
                this.updateStatusBar('Disconnected');
                break;
        }
    }

    private updateStatusBar(text: string): void {
        this.statusBarItem.text = `$(plug) Naso: ${text}`;
        this.statusBarItem.tooltip = `Naso Language Server: ${text}. Click to restart.`;
    }

    private log(message: string): void {
        const timestamp = new Date().toISOString();
        this.outputChannel.appendLine(`[${timestamp}] ${message}`);
    }

    public getClient(): LanguageClient | undefined {
        return this.client;
    }

    public isRunning(): boolean {
        return this.client?.state === LanguageClient.State.Running;
    }

    public updateConfig(): void {
        const newConfig = this.loadConfig();
        const serverPathChanged = newConfig.serverPath !== this.config.serverPath;
        const enableChanged = newConfig.enable !== this.config.enable;
        
        this.config = newConfig;
        
        if (enableChanged) {
            if (newConfig.enable) {
                this.start();
            } else {
                this.stop();
            }
        } else if (serverPathChanged && this.config.enable) {
            this.restart();
        }
    }

    public dispose(): void {
        this.stop();
        this.outputChannel.dispose();
        this.statusBarItem.dispose();
    }
}