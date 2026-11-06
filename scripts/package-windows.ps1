# Windows Packaging Automation Script (ADR-0030)
# Packages all 12 monorepo standalone applications for Windows
param (
    [string]$Target = $( (rustc -vV | Select-String "host:").ToString().Split(':')[1].Trim() ),
    [string]$Version = "0.1.0",
    [string]$OutputDir = "dist/windows",
    [string]$AppName = ""
)

$ErrorActionPreference = "Stop"
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Packaging Monorepo Apps for Windows ($Target) v$Version" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if ($AppName -ne "") {
    $Apps = @($AppName)
} else {
    $Apps = @("devops-secret-vault", "ai-file-explorer", "universal-log-viewer", "universal-live-translator", "pro-av-editor", "personal-knowledge-graph", "api-protocol-studio", "parquet-analytics-workbench", "dfir-triage-kit", "screen-recorder-studio", "signal-telemetry-oscilloscope", "audio-mastering-studio", "qr-code-studio")
}

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# 0. Build Embedded Svelte 5 Desktop UI Bundles
Write-Host "[0/4] Building Svelte 5 desktop UI bundles for all applications..." -ForegroundColor Yellow
foreach ($app in $Apps) {
    $uiDir = "apps/$app/ui"
    if (Test-Path "$uiDir/package.json") {
        npm --prefix $uiDir run build
    }
}

# 1. Compile Release Binaries
Write-Host "[1/4] Building release binaries for target: $Target..." -ForegroundColor Yellow
if ($AppName -ne "") {
    cargo build --release --target $Target -p $AppName
} else {
    cargo build --workspace --release --target $Target
}

# 2. Verify Embedded UI & IPC Self-Test and Package Portable ZIP Archives
Write-Host "[2/4] Verifying --self-test and assembling portable ZIP archives..." -ForegroundColor Yellow
foreach ($app in $Apps) {
    $binName = "$app.exe"
    $srcPath = "target/$Target/release/$binName"
    if (-not (Test-Path $srcPath) -and (Test-Path "target/release/$binName")) {
        $srcPath = "target/release/$binName"
    }
    if (Test-Path $srcPath) {
        Write-Host "  -> Running live GUI & IPC self-test: $srcPath --self-test" -ForegroundColor Cyan
        & $srcPath --self-test
        if ($LASTEXITCODE -ne 0) {
            throw "Self-test failed for $srcPath"
        }
        $stageDir = "$OutputDir/stage/$app"
        New-Item -ItemType Directory -Force -Path $stageDir | Out-Null
        Copy-Item $srcPath -Destination "$stageDir/$binName"
        
        # Include license, readme, and portable marker flag
        New-Item -ItemType File -Force -Path "$stageDir/portable.flag" | Out-Null
        if (Test-Path "LICENSE") { Copy-Item "LICENSE" -Destination $stageDir }
        if (Test-Path "README.md") { Copy-Item "README.md" -Destination $stageDir }

        # Generate CycloneDX SBOM for the application
        & "$PSScriptRoot/generate-sbom.ps1" -AppName $app -OutputDir "$stageDir"
        if (Test-Path "$stageDir/$app-sbom.json") {
            Copy-Item "$stageDir/$app-sbom.json" -Destination "$OutputDir/$app-sbom.json" -Force
        }

        # Also copy standalone executable directly into $OutputDir for direct launch
        Copy-Item $srcPath -Destination "$OutputDir/$binName" -Force

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
Get-ChildItem -Path $OutputDir -File | Where-Object { ($_.Extension -in @(".zip", ".exe", ".json") -or $_.Name.EndsWith(".wxs")) -and -not $_.Name.EndsWith(".sha256") } | ForEach-Object {
    $hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash.ToLower()
    $hashFile = "$($_.FullName).sha256"
    "$hash  $($_.Name)" | Set-Content -Path $hashFile -Encoding ASCII
    Write-Host "  -> SHA256($($_.Name)): $hash" -ForegroundColor Green
}

Write-Host "Windows packaging completed successfully!" -ForegroundColor Cyan
