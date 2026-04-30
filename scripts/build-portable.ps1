# AI Image V2 Portable Build Script
# This script builds the app and creates a portable version

param(
    [string]$OutputDir = "..\dist-portable"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "AI Image V2 Portable Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Build frontend
Write-Host "[1/5] Building frontend..." -ForegroundColor Yellow
Set-Location $PSScriptRoot\..
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Error "Frontend build failed"
    exit 1
}

# 2. Build Rust
Write-Host "[2/5] Building Rust backend..." -ForegroundColor Yellow
Set-Location $PSScriptRoot\..\src-tauri
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Rust build failed"
    exit 1
}

# 3. Prepare output directory
Write-Host "[3/5] Preparing output directory..." -ForegroundColor Yellow
$AppName = "AI Image V2"
$PortableDir = "$PSScriptRoot\$OutputDir\$AppName"
if (Test-Path $PortableDir) {
    Remove-Item -Path $PortableDir -Recurse -Force
}
New-Item -ItemType Directory -Path $PortableDir -Force | Out-Null

# 4. Copy files
Write-Host "[4/5] Copying application files..." -ForegroundColor Yellow
$SourceDir = "$PSScriptRoot\..\src-tauri\target\release"

# Copy executable
Copy-Item -Path "$SourceDir\ai-image-v2.exe" -Destination "$PortableDir\AI Image V2.exe" -Force

# Copy icon files
$IconsDir = "$PortableDir\icons"
New-Item -ItemType Directory -Path $IconsDir -Force | Out-Null
Copy-Item -Path "$PSScriptRoot\..\src-tauri\icons\*.png" -Destination $IconsDir -Force
Copy-Item -Path "$PSScriptRoot\..\src-tauri\icons\*.ico" -Destination $IconsDir -Force

# 5. Create startup script
Write-Host "[5/5] Creating startup script..." -ForegroundColor Yellow
$StartScript = '@echo off' + "`n" + 'chcp 65001 >nul' + "`n" + 'echo Starting AI Image V2...' + "`n" + 'start "" "%~dp0AI Image V2.exe"'
Set-Content -Path "$PortableDir\Start.bat" -Value $StartScript -Encoding UTF8

# Create readme file
$BuildTime = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
$Readme = @"
AI Image V2 Portable Version
============================

Usage:
1. Run "AI Image V2.exe" directly
2. Or run "Start.bat" to launch

Notes:
- This is a portable version, no installation required
- Config files will be saved in %USERPROFILE%\.ai-image
- WebView2 runtime may be downloaded on first run

Requirements:
- Windows 10/11 64-bit
- WebView2 runtime (auto-download on first run)

Version: 2.0.0
Build Time: $BuildTime
"@
Set-Content -Path "$PortableDir\README.txt" -Value $Readme -Encoding UTF8

# Output result
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Portable build completed!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "Output: $PortableDir" -ForegroundColor White
Write-Host ""
Write-Host "Files:" -ForegroundColor Cyan
Get-ChildItem $PortableDir | ForEach-Object {
    $size = if ($_.Length -gt 1MB) { "{0:N2} MB" -f ($_.Length / 1MB) } else { "{0:N2} KB" -f ($_.Length / 1KB) }
    Write-Host "  - $($_.Name) ($size)" -ForegroundColor Gray
}
Write-Host ""
Write-Host "Tip: Zip the '$AppName' folder and distribute to users" -ForegroundColor Yellow
