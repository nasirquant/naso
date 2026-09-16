# Naso Language - Automated Installer Scripts

This directory contains the official bootstrap scripts for installing the Naso programming language toolchain (`naso` compiler and `naso-lsp` language server) on Linux, macOS, and Windows.

## Quick Installation

### Linux / macOS (POSIX Shell)

```bash
curl -fsSL https://nasolang.org/install.sh | sh
```

**What it does:**
1. Auto-detects your architecture (`x86_64` or `aarch64`) and OS (`Linux` or `macOS`)
2. Fetches the latest release version from GitHub Releases
3. Downloads `naso` and `naso-lsp` tarballs with SHA256 checksum verification
4. Installs binaries to `~/.naso/bin`
5. Updates `PATH` in `~/.bashrc`, `~/.zshrc`, and `~/.profile`

**After installation:**
```bash
# Restart your shell or source your rc file
source ~/.bashrc    # or ~/.zshrc

# Verify installation
naso --version
naso-lsp --version
```

### Windows (PowerShell)

```powershell
irm https://nasolang.org/install.ps1 | iex
```

**What it does:**
1. Detects Windows `x86_64` architecture
2. Fetches the latest release version from GitHub Releases
3. Downloads `naso` and `naso-lsp` tarballs with SHA256 checksum verification
4. Installs binaries to `%USERPROFILE%\.naso\bin`
5. Adds the install directory to your **User** PATH environment variable

**After installation:**
```powershell
# Restart PowerShell or open a new session

# Verify installation
naso --version
naso-lsp --version
```

## Manual Installation

If you prefer not to pipe to shell, you can download and run the scripts manually:

### Linux / macOS
```bash
# Download
curl -fsSL -o install.sh https://nasolang.org/install.sh
chmod +x install.sh

# Inspect (recommended)
cat install.sh

# Run
./install.sh
```

### Windows
```powershell
# Download
irm https://nasolang.org/install.ps1 -OutFile install.ps1

# Inspect (recommended)
cat install.ps1

# Run
.\install.ps1
```

## Verification

Both installers verify SHA256 checksums against the `SHA256SUMS` file published with each GitHub Release. If checksums don't match, the installation aborts.

## Supported Platforms

| OS | Architecture | Binary Target |
|---|---|---|
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| Linux | aarch64 | `aarch64-unknown-linux-gnu` |
| macOS | x86_64 | `x86_64-apple-darwin` |
| macOS | aarch64 (Apple Silicon) | `aarch64-apple-darwin` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` |

## Custom Installation Directory

To install to a custom location, set the `NASO_INSTALL_DIR` environment variable before running:

### Linux / macOS
```bash
NASO_INSTALL_DIR="$HOME/custom/bin" curl -fsSL https://nasolang.org/install.sh | sh
```

### Windows
```powershell
$env:NASO_INSTALL_DIR = "$env:USERPROFILE\custom\bin"
irm https://nasolang.org/install.ps1 | iex
```

## Uninstallation

### Linux / macOS
```bash
# Remove binaries
rm -rf ~/.naso/bin

# Remove PATH entries from shell rc files (manual edit required)
# Edit ~/.bashrc, ~/.zshrc, ~/.profile and remove the Naso PATH line
```

### Windows
```powershell
# Remove binaries
Remove-Item -Recurse -Force "$env:USERPROFILE\.naso\bin"

# Remove from User PATH (manual via System Properties > Environment Variables)
```

## Release Channels

By default, installers fetch the **latest stable release**. To install a specific version, set `NASO_VERSION`:

### Linux / macOS
```bash
NASO_VERSION="v0.9.0" curl -fsSL https://nasolang.org/install.sh | sh
```

### Windows
```powershell
$env:NASO_VERSION = "v0.9.0"
irm https://nasolang.org/install.ps1 | iex
```

## Troubleshooting

### "Command not found" after installation
- **Linux/macOS**: Restart your shell or run `source ~/.bashrc` (or `~/.zshrc`)
- **Windows**: Open a new PowerShell window

### Checksum verification failed
This indicates a corrupted download or potential security issue. Try again, and if it persists, report an issue at https://github.com/nasolang/naso/issues

### Permission denied (Linux/macOS)
Ensure the install directory is writable:
```bash
mkdir -p ~/.naso/bin
chmod 755 ~/.naso/bin
```

### Execution policy error (Windows)
If PowerShell blocks script execution:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
# Then re-run the installer
```

## Security

- All downloads are served over HTTPS from GitHub Releases
- SHA256 checksums are verified against the release's `SHA256SUMS` file
- No elevated privileges required (installs to user directory)
- Scripts are open source and auditable at https://github.com/nasolang/naso/tree/main/scripts

## Related Links

- **Website**: https://nasolang.org
- **GitHub**: https://github.com/nasolang/naso
- **Releases**: https://github.com/nasolang/naso/releases
- **Documentation**: https://nasolang.org/docs
- **Issues**: https://github.com/nasolang/naso/issues