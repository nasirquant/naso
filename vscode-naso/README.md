# Naso Language Support for VS Code

This extension provides rich language support for the [Naso programming language](https://nasolang.org) in Visual Studio Code.

## Features

### Syntax Highlighting
Full syntax highlighting for Naso source files (`.naso`).

### QTT Hover Tooltips
Hover over identifiers to see detailed type information from the Query Type Table (QTT), showing inferred types and constraints.

### Real-time SMT Diagnostics
Get instant feedback on your code with SMT-based diagnostics that check for:
- Type errors
- Logic inconsistencies
- Constraint violations
- Unreachable code paths

### Language Server Integration
Connects to the `naso-lsp` language server for:
- Go to Definition
- Find References
- Symbol Search
- Code Actions
- Completion

### Binary Resolution
The extension automatically locates or downloads the Naso binary:
1. Checks system PATH for `naso` executable
2. Looks in `.naso/bin/` relative to workspace
3. Auto-fetches from https://nasolang.org if not found

## Requirements

This extension requires the Naso language tools to be installed. The extension will attempt to automatically install the Naso binary if not found in your PATH.

## Extension Settings

This extension contributes the following settings:

* `naso.enable`: Enable/disable the Naso language support
* `naso.trace.server`: Trace the communication between VS Code and the Naso language server
* `naso.lsp.path`: Custom path to the naso-lsp executable

## Known Issues

Please report any issues at [nasirquant/naso](https://github.com/nasirquant/naso/issues).

## Release Notes

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes.

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on how to submit pull requests.

## License

This extension is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Thanks to the Naso language team and the VS Code extension ecosystem.