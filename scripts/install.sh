#!/bin/sh
# Naso Language Installer - POSIX-compliant shell script
# Usage: curl -fsSL https://nasolang.org/install.sh | sh

set -eu

# Configuration
REPO_OWNER="nasolang"
REPO_NAME="naso"
INSTALL_DIR="${NASO_INSTALL_DIR:-${HOME}/.naso/bin}"
RELEASE_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
DOWNLOAD_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    printf "${BLUE}[INFO]${NC} %s\n" "$1"
}

log_success() {
    printf "${GREEN}[SUCCESS]${NC} %s\n" "$1"
}

log_warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

log_error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1" >&2
}

# Detect architecture
detect_arch() {
    ARCH=$(uname -m)
    case "${ARCH}" in
        x86_64|amd64)
            NASO_ARCH="x86_64"
            ;;
        aarch64|arm64)
            NASO_ARCH="aarch64"
            ;;
        *)
            log_error "Unsupported architecture: ${ARCH}"
            exit 1
            ;;
    esac
    log_info "Detected architecture: ${NASO_ARCH}"
}

# Detect OS
detect_os() {
    OS=$(uname -s)
    case "${OS}" in
        Linux)
            NASO_OS="unknown-linux-gnu"
            ;;
        Darwin)
            NASO_OS="apple-darwin"
            ;;
        *)
            log_error "Unsupported operating system: ${OS}"
            exit 1
            ;;
    esac
    log_info "Detected OS: ${NASO_OS}"
}

# Get latest release version from GitHub API
get_latest_version() {
    log_info "Fetching latest release version..."
    VERSION=$(curl -fsSL "${RELEASE_URL}" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": "\([^"]*\)".*/\1/')
    if [ -z "${VERSION}" ]; then
        log_error "Failed to fetch latest version"
        exit 1
    fi
    log_info "Latest version: ${VERSION}"
}

# Download file with SHA256 verification
download_and_verify() {
    local url="$1"
    local output="$2"
    local expected_sha256="$3"
    
    log_info "Downloading $(basename "${output}")..."
    curl -fsSL -o "${output}" "${url}"
    
    if [ -n "${expected_sha256}" ]; then
        log_info "Verifying SHA256 checksum..."
        ACTUAL_SHA256=$(sha256sum "${output}" | cut -d' ' -f1)
        if [ "${ACTUAL_SHA256}" != "${expected_sha256}" ]; then
            log_error "Checksum mismatch for $(basename "${output}")"
            log_error "Expected: ${expected_sha256}"
            log_error "Actual:   ${ACTUAL_SHA256}"
            rm -f "${output}"
            exit 1
        fi
        log_success "Checksum verified"
    fi
}

# Fetch SHA256 checksums from release assets
fetch_checksums() {
    log_info "Fetching checksums..."
    CHECKSUM_URL="${DOWNLOAD_BASE}/${VERSION}/SHA256SUMS"
    CHECKSUMS=$(curl -fsSL "${CHECKSUM_URL}" 2>/dev/null || echo "")
    
    if [ -z "${CHECKSUMS}" ]; then
        log_warn "No SHA256SUMS file found, skipping checksum verification"
        NASO_SHA256=""
        LSP_SHA256=""
    else
        NASO_SHA256=$(echo "${CHECKSUMS}" | grep "naso-${VERSION}-${NASO_ARCH}-${NASO_OS}\.tar\.gz" | cut -d' ' -f1)
        LSP_SHA256=$(echo "${CHECKSUMS}" | grep "naso-lsp-${VERSION}-${NASO_ARCH}-${NASO_OS}\.tar\.gz" | cut -d' ' -f1)
        log_info "Found checksums for naso and naso-lsp"
    fi
}

# Install binary from tarball
install_binary() {
    local name="$1"
    local sha256="$2"
    local tarball="${name}-${VERSION}-${NASO_ARCH}-${NASO_OS}.tar.gz"
    local url="${DOWNLOAD_BASE}/${VERSION}/${tarball}"
    local temp_dir=$(mktemp -d)
    
    download_and_verify "${url}" "${temp_dir}/${tarball}" "${sha256}"
    
    log_info "Extracting ${name}..."
    tar -xzf "${temp_dir}/${tarball}" -C "${temp_dir}"
    
    # Find the binary (could be in root or subdirectory)
    BINARY=$(find "${temp_dir}" -type f -name "${name}" -perm -u+x | head -1)
    if [ -z "${BINARY}" ]; then
        log_error "Binary ${name} not found in tarball"
        rm -rf "${temp_dir}"
        exit 1
    fi
    
    mkdir -p "${INSTALL_DIR}"
    cp "${BINARY}" "${INSTALL_DIR}/${name}"
    chmod +x "${INSTALL_DIR}/${name}"
    log_success "Installed ${name} to ${INSTALL_DIR}"
    
    rm -rf "${temp_dir}"
}

# Update shell PATH in rc files
update_path() {
    local path_entry="export PATH=\"${INSTALL_DIR}:\${PATH}\""
    local updated=false
    
    for rc_file in "${HOME}/.bashrc" "${HOME}/.zshrc" "${HOME}/.profile"; do
        if [ -f "${rc_file}" ]; then
            if ! grep -q "${INSTALL_DIR}" "${rc_file}"; then
                echo "" >> "${rc_file}"
                echo "# Added by Naso installer" >> "${rc_file}"
                echo "${path_entry}" >> "${rc_file}"
                log_success "Updated PATH in ${rc_file}"
                updated=true
            else
                log_info "PATH already configured in ${rc_file}"
            fi
        fi
    done
    
    if [ "${updated}" = "true" ]; then
        log_warn "Restart your shell or run: source ~/.bashrc (or ~/.zshrc)"
    fi
}

# Main installation flow
main() {
    log_info "Starting Naso installation..."
    
    detect_arch
    detect_os
    get_latest_version
    fetch_checksums
    
    install_binary "naso" "${NASO_SHA256}"
    install_binary "naso-lsp" "${LSP_SHA256}"
    
    update_path
    
    log_success "Naso ${VERSION} installed successfully!"
    log_info "Binaries installed to: ${INSTALL_DIR}"
    log_info "Run 'naso --version' to verify installation"
}

main "$@"