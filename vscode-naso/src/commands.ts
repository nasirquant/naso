import * as vscode from 'vscode';

export interface BuildCommandOptions {
    target?: 'llvm' | 'qir' | 'jit';
    release?: boolean;
    args?: string[];
}

export class NasoCommands {
    private terminal: vscode.Terminal | undefined;
    private outputChannel: vscode.OutputChannel;

    constructor(outputChannel: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
    }

    private getOrCreateTerminal(): vscode.Terminal {
        if (!this.terminal || this.terminal.exitStatus !== undefined) {
            this.terminal = vscode.window.createTerminal({
                name: 'Naso Build',
                cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
            });
        }
        return this.terminal;
    }

    private async runCommand(command: string, args: string[] = [], options: BuildCommandOptions = {}): Promise<void> {
        const terminal = this.getOrCreateTerminal();
        const fullCommand = [command, ...args].join(' ');
        
        this.outputChannel.appendLine(`$ ${fullCommand}`);
        this.outputChannel.show(true);
        terminal.show(true);
        terminal.sendText(fullCommand);
    }

    public async build(options: BuildCommandOptions = {}): Promise<void> {
        const config = vscode.workspace.getConfiguration('naso.build');
        const target = options.target || config.get<string>('target', 'llvm');
        const release = options.release ?? true;
        
        const args = ['build'];
        if (target !== 'llvm') {
            args.push('--target', target);
        }
        if (release) {
            args.push('--release');
        }
        if (options.args) {
            args.push(...options.args);
        }

        await this.runCommand('naso', args, options);
    }

    public async run(options: BuildCommandOptions = {}): Promise<void> {
        const config = vscode.workspace.getConfiguration('naso.build');
        const target = options.target || config.get<string>('target', 'llvm');
        
        const args = ['run'];
        if (target !== 'llvm') {
            args.push('--target', target);
        }
        if (options.args) {
            args.push(...options.args);
        }

        await this.runCommand('naso', args, options);
    }

    public async emitQir(options: BuildCommandOptions = {}): Promise<void> {
        const args = ['emit', 'qir'];
        if (options.args) {
            args.push(...options.args);
        }
        await this.runCommand('naso', args, options);
    }

    public async emitLlvm(options: BuildCommandOptions = {}): Promise<void> {
        const args = ['emit', 'llvm'];
        if (options.args) {
            args.push(...options.args);
        }
        await this.runCommand('naso', args, options);
    }

    public async check(): Promise<void> {
        await this.runCommand('naso', ['check']);
    }

    public async test(): Promise<void> {
        await this.runCommand('naso', ['test']);
    }

    public async fmt(): Promise<void> {
        await this.runCommand('naso', ['fmt']);
    }

    public async clippy(): Promise<void> {
        await this.runCommand('naso', ['clippy']);
    }

    public async doc(): Promise<void> {
        await this.runCommand('naso', ['doc']);
    }

    public dispose(): void {
        if (this.terminal) {
            this.terminal.dispose();
            this.terminal = undefined;
        }
    }
}

export function registerCommands(context: vscode.ExtensionContext, commands: NasoCommands): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('naso.build', (options?: BuildCommandOptions) => commands.build(options)),
        vscode.commands.registerCommand('naso.run', (options?: BuildCommandOptions) => commands.run(options)),
        vscode.commands.registerCommand('naso.emitQir', (options?: BuildCommandOptions) => commands.emitQir(options)),
        vscode.commands.registerCommand('naso.emitLlvm', (options?: BuildCommandOptions) => commands.emitLlvm(options)),
        vscode.commands.registerCommand('naso.check', () => commands.check()),
        vscode.commands.registerCommand('naso.test', () => commands.test()),
        vscode.commands.registerCommand('naso.fmt', () => commands.fmt()),
        vscode.commands.registerCommand('naso.clippy', () => commands.clippy()),
        vscode.commands.registerCommand('naso.doc', () => commands.doc())
    );
}