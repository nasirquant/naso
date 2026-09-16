import * as path from 'path';
import * as vscode from 'vscode';
import { runTests } from '@vscode/test-electron';

async function main() {
    try {
        // The folder containing the Extension Manifest package.json
        // Passed to `--extensionDevelopmentPath`
        const extensionDevelopmentPath = path.resolve(__dirname, '../../');

        // The path to the extension test script
        // Passed to `--extensionTestsPath`
        const extensionTestsPath = path.resolve(__dirname, './suite/index');

        // Download VS Code, unzip it and run the integration test
        await runTests({
            extensionDevelopmentPath,
            extensionTestsPath,
            launchArgs: [
                // Disable extensions to avoid interference
                '--disable-extensions',
                // Open test fixture files
                path.resolve(__dirname, '../fixtures/basic.naso'),
                path.resolve(__dirname, '../fixtures/quantum.naso'),
                path.resolve(__dirname, '../fixtures/tensor.naso'),
                path.resolve(__dirname, '../fixtures/linearity.naso')
            ]
        });
    } catch (err) {
        console.error('Failed to run tests:', err);
        process.exit(1);
    }
}

main();