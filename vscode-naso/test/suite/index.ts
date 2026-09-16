import * as vscode from 'vscode';
import { runAllTests } from './integration.test';

export async function run(): Promise<void> {
    // Wait for extension to fully activate
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Run all integration tests
    await runAllTests();
}