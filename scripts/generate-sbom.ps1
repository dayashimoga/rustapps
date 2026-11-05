# PowerShell SBOM Generation Script (ADR-0030)
# Generates compliant CycloneDX 1.5 JSON Software Bill of Materials (SBOM)
param (
    [string]$AppName = "",
    [string]$OutputDir = "target/sbom"
)

Write-Host "Generating CycloneDX Software Bill of Materials (SBOM)..." -ForegroundColor Cyan
$ErrorActionPreference = "Stop"

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}

$outputPattern = "$OutputDir/sbom-cyclonedx.json"
if ($AppName -ne "") {
    $outputPattern = "$OutputDir/$AppName-sbom.json"
}

$generated = $false
try {
    $ver = cargo cyclonedx --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        if ($AppName -ne "") {
            cargo cyclonedx --package $AppName --format json --output-pattern $outputPattern
        } else {
            cargo cyclonedx --format json --output-pattern $outputPattern
        }
        if ($LASTEXITCODE -eq 0 -and (Test-Path $outputPattern)) {
            $generated = $true
            Write-Host "SBOM generated via cargo-cyclonedx at $outputPattern" -ForegroundColor Green
        }
    }
} catch {
    # Fallback to internal generator
}

if (-not $generated) {
    Write-Host "cargo-cyclonedx not available or failed; generating compliant offline CycloneDX 1.5 SBOM..." -ForegroundColor Yellow
    
    $lockLines = Get-Content "Cargo.lock"
    $components = [System.Collections.Generic.List[PSObject]]::new()
    $currentName = ""
    $currentVersion = ""

    foreach ($line in $lockLines) {
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith("name = ")) {
            $currentName = $trimmed.Substring(7).Trim('"')
        } elseif ($trimmed.StartsWith("version = ") -and $currentName -ne "") {
            $currentVersion = $trimmed.Substring(10).Trim('"')
            $components.Add([PSCustomObject]@{
                type = "library"
                name = $currentName
                version = $currentVersion
                purl = "pkg:cargo/$currentName@$currentVersion"
                "bom-ref" = "pkg:cargo/$currentName@$currentVersion"
            })
            $currentName = ""
            $currentVersion = ""
        }
    }

    $targetName = if ($AppName -ne "") { $AppName } else { "rustapps-workspace" }
    $uuid = [System.Guid]::NewGuid().ToString()
    $now = [System.DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")

    $sbom = [PSCustomObject]@{
        bomFormat = "CycloneDX"
        specVersion = "1.5"
        serialNumber = "urn:uuid:$uuid"
        version = 1
        metadata = [PSCustomObject]@{
            timestamp = $now
            tools = @(
                [PSCustomObject]@{
                    vendor = "Offline Engineering"
                    name = "cargo-sbom-generator"
                    version = "1.5.0"
                }
            )
            component = [PSCustomObject]@{
                type = "application"
                name = $targetName
                version = "0.1.0"
            }
        }
        components = $components
    }

    $json = $sbom | ConvertTo-Json -Depth 10
    Set-Content -Path $outputPattern -Value $json -Encoding UTF8
    Write-Host "Offline CycloneDX 1.5 SBOM generated with $($components.Count) components at $outputPattern" -ForegroundColor Green
}
