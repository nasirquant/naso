<# 
.SYNOPSIS
    Naso Language Installer for Windows PowerShell
.DESCRIPTION
    Downloads and installs naso and naso-lsp binaries from GitHub Releases
.EXAMPLE
    irm https://nasolang.org/install.ps1 | iex
#>

# Configuration
$RepoOwner = "nasolang"
$RepoName = "naso"
$InstallDir = $env:NASO_INSTALL_DIR
if (-not $InstallDir) { $InstallDir = "$env:USERPROFILE\.naso\bin" }
$ReleaseApiUrl = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
$DownloadBase = "https://github.com/$RepoOwner/$RepoName/releases/download"

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $colors = @{
        INFO = "Cyan"
        SUCCESS = "Green"
        WARN = "Yellow"
        ERROR = "Red"
    }
    $color = $colors[$Level]
    if (-not $color) { $color = "White" }
    Write-Host "[$Level] $Message" -ForegroundColor $color
}

function Detect-Architecture {
    $arch = [Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    if ($arch -eq "AMD64") {
        return "x86_64"
    }
    elseif ($arch -eq "ARM64") {
        return "aarch64"
    }
    else {
        Write-Log "Unsupported architecture: $arch" -Level ERROR
        exit 1
    }
}

function Get-LatestVersion {
    Write-Log "Fetching latest release version..."
    try {
        $response = Invoke-RestMethod -Uri $ReleaseApiUrl -UseBasicParsing
        $version = $response.tag_name
        if (-not $version) {
            Write-Log "Failed to fetch latest version" -Level ERROR
            exit 1
        }
        Write-Log "Latest version: $version"
        return $version
    }
    catch {
        Write-Log "Error fetching release: $_" -Level ERROR
        exit 1
    }
}

function Fetch-Checksums {
    param([string]$Version, [string]$Arch)
    $os = "pc-windows-msvc"
    $checksumUrl = "$DownloadBase/$Version/SHA256SUMS"
    
    Write-Log "Fetching checksums..."
    try {
        $checksums = Invoke-RestMethod -Uri $checksumUrl -UseBasicParsing
        $nasoPattern = "naso-$Version-$Arch-$os\.tar\.gz"
        $lspPattern = "naso-lsp-$Version-$Arch-$os\.tar\.gz"
        
        $nasoSha256 = ($checksums | Where-Object { $_ -match $nasoPattern }) -split '\s+' | Select-Object -First 1
        $lspSha256 = ($checksums | Where-Object { $_ -match $lspPattern }) -split '\s+' | Select-Object -First 1
        
        if ($nasoSha256 -and $lspSha256) {
            Write-Log "Found checksums for naso and naso-lsp"
            return @{ Naso = $nasoSha256; Lsp = $lspSha256 }
        }
        else {
            Write-Log "Checksums not found for this platform, skipping verification" -Level WARN
            return @{ Naso = ""; Lsp = "" }
        }
    }
    catch {
        Write-Log "No SHA256SUMS file found, skipping checksum verification" -Level WARN
        return @{ Naso = ""; Lsp = "" }
    }
}

function Download-AndVerify {
    param([string]$Url, [string]$OutputPath, [string]$ExpectedSha256)
    
    Write-Log "Downloading $(Split-Path $OutputPath -Leaf)..."
    try {
        Invoke-WebRequest -Uri $Url -OutFile $OutputPath -UseBasicParsing
    }
    catch {
        Write-Log "Download failed: $_" -Level ERROR
        exit 1
    }
    
    if ($ExpectedSha256) {
        Write-Log "Verifying SHA256 checksum..."
        $actualSha256 = (Get-FileHash -Path $OutputPath -Algorithm SHA256).Hash.ToLower()
        $expectedSha256 = $ExpectedSha256.ToLower()
        
        if ($actualSha256 -ne $expectedSha256) {
            Write-Log "Checksum mismatch for $(Split-Path $OutputPath -Leaf)" -Level ERROR
            Write-Log "Expected: $expectedSha256" -Level ERROR
            Write-Log "Actual:   $actualSha256" -Level ERROR
            Remove-Item -Force $OutputPath -ErrorAction SilentlyContinue
            exit 1
        }
        Write-Log "Checksum verified" -Level SUCCESS
    }
}

function Install-Binary {
    param([string]$Name, [string]$Sha256, [string]$Version, [string]$Arch)
    $os = "pc-windows-msvc"
    $tarball = "$Name-$Version-$Arch-$os.tar.gz"
    $url = "$DownloadBase/$Version/$tarball"
    $tempDir = [System.IO.Path]::GetTempPath() + "naso_install_" + [System.Guid]::NewGuid().ToString()
    
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    
    Download-AndVerify -Url $Url -OutputPath "$tempDir\$tarball" -ExpectedSha256 $Sha256
    
    Write-Log "Extracting $Name..."
    tar -xzf "$tempDir\$tarball" -C $tempDir
    
    # Find the binary (could be in root or subdirectory)
    $binary = Get-ChildItem -Path $tempDir -Recurse -Filter "$Name.exe" -File | Select-Object -First 1
    if (-not $binary) {
        $binary = Get-ChildItem -Path $tempDir -Recurse -Filter "$Name" -File | Select-Object -First 1
    }
    
    if (-not $binary) {
        Write-Log "Binary $Name not found in tarball" -Level ERROR
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        exit 1
    }
    
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -Path $binary.FullName -Destination "$InstallDir\$Name.exe" -Force
    Write-Log "Installed $Name to $InstallDir" -Level SUCCESS
    
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
}

function Update-UserPath {
    $pathEntry = $InstallDir
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    if ($currentPath -notlike "*$pathEntry*") {
        $newPath = "$currentPath;$pathEntry"
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Log "Added $pathEntry to User PATH" -Level SUCCESS
        Write-Log "Restart your PowerShell session for changes to take effect" -Level WARN
    }
    else {
        Write-Log "PATH already configured" -Level INFO
    }
}

# Main installation flow
Write-Log "Starting Naso installation..."

$arch = Detect-Architecture
$version = Get-LatestVersion
$checksums = Fetch-Checksums -Version $version -Arch $arch

Install-Binary -Name "naso" -Sha256 $checksums.Naso -Version $version -Arch $arch
Install-Binary -Name "naso-lsp" -Sha256 $checksums.Lsp -Version $version -Arch $arch

Update-UserPath

Write-Log "Naso $version installed successfully!" -Level SUCCESS
Write-Log "Binaries installed to: $InstallDir"
Write-Log "Run 'naso --version' to verify installation"