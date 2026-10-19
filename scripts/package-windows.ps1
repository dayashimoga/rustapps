# Windows Packaging Automation Script (ADR-0030)
# Packages DevOps Secret Vault, AI File Explorer, and Universal Log Viewer for Windows
param (
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$Version = "0.1.0",
    [string]$OutputDir = "dist/windows"
)

$ErrorActionPreference = "Stop"
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Packaging Monorepo Apps for Windows ($Target) v$Version" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$Apps = @("devops-secret-vault", "ai-file-explorer", "universal-log-viewer")

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# 1. Compile Release Binaries
Write-Host "[1/4] Building release binaries for target: $Target..." -ForegroundColor Yellow
cargo build --workspace --release --target $Target

# 2. Package Portable ZIP Archives
Write-Host "[2/4] Assembling portable ZIP archives..." -ForegroundColor Yellow
foreach ($app in $Apps) {
    $binName = "$app.exe"
    $srcPath = "target/$Target/release/$binName"
    if (Test-Path $srcPath) {
        $stageDir = "$OutputDir/stage/$app"
        New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
        Copy-Item $srcPath -Destination "$stageDir/$binName"
        
        # Include license, readme, and portable marker flag
        New-Item -ItemType File -Force -Path "$stageDir/portable.flag" | Out-Null
        if (Test-Path "LICENSE") { Copy-Item "LICENSE" -Destination $stageDir }
        if (Test-Path "README.md") { Copy-Item "README.md" -Destination $stageDir }

        $zipName = "${app}_${Version}_${Target}_portable.zip"
        $zipPath = "$OutputDir/$zipName"
        if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
        Compress-Archive -Path "$stageDir/*" -DestinationPath $zipPath -Force
        Write-Host "  -> Created portable bundle: $zipName" -ForegroundColor Green
        Remove-Item -Recurse -Force "$OutputDir/stage"
    } else {
        Write-Warning "Binary not found: $srcPath"
    }
}

# 3. Generate WiX MSI Installer Template
Write-Host "[3/4] Generating WiX MSI installer configuration..." -ForegroundColor Yellow
foreach ($app in $Apps) {
    $wxsContent = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product Id="*" Name="$app" Language="1033" Version="$Version" Manufacturer="OfflineApps" UpgradeCode="{A1B2C3D4-E5F6-4A7B-8C9D-0E1F2A3B4C5D}">
        <Package InstallerVersion="500" Compressed="yes" InstallScope="perMachine" />
        <MajorUpgrade DowngradeErrorMessage="A newer version is already installed." />
        <MediaTemplate EmbedCab="yes" />
        <Feature Id="ProductFeature" Title="$app" Level="1">
            <ComponentRef Id="MainExecutable" />
        </Feature>
        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="ProgramFilesFolder">
                <Directory Id="INSTALLFOLDER" Name="$app">
                    <Component Id="MainExecutable" Guid="*">
                        <File Id="AppExe" Source="target/$Target/release/$app.exe" KeyPath="yes" />
                    </Component>
                </Directory>
            </Directory>
        </Directory>
    </Product>
</Wix>
"@
    $wxsPath = "$OutputDir/${app}_installer.wxs"
    Set-Content -Path $wxsPath -Value $wxsContent -Encoding UTF8
    Write-Host "  -> Generated WiX manifest: $wxsPath" -ForegroundColor Green
}

# 4. Compute Cryptographic Checksums (SHA-256)
Write-Host "[4/4] Computing SHA-256 cryptographic checksums..." -ForegroundColor Yellow
Get-ChildItem -Path $OutputDir -File -Filter "*.zip" | ForEach-Object {
    $hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash.ToLower()
    $hashFile = "$($_.FullName).sha256"
    "$hash  $($_.Name)" | Set-Content -Path $hashFile -Encoding ASCII
    Write-Host "  -> SHA256($($_.Name)): $hash" -ForegroundColor Green
}

Write-Host "Windows packaging completed successfully!" -ForegroundColor Cyan
