<#
.SYNOPSIS
    Generate SBOM (CycloneDX + SPDX) for the RNFC workspace.
.DESCRIPTION
    Runs cargo-sbom and produces both CycloneDX 1.6 JSON and SPDX 2.3 JSON.
    Outputs go to security/sbom/ with the current git tag (if available).
#>

param(
    [string]$OutDir = "security\sbom"
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Determine version tag
$tag = git describe --tags --always --dirty 2>$null
if (-not $tag) { $tag = "dev" }
Write-Host "Version: $tag" -ForegroundColor Cyan

# 1. CycloneDX 1.6
Write-Host "[1/2] Generating CycloneDX 1.6 SBOM..." -ForegroundColor Cyan
$cdx = Join-Path $OutDir "rnfc-$tag.cdx.json"
cargo sbom --output-format cyclone_dx_json_1_6 2>$null | Out-File -FilePath $cdx -Encoding UTF8
if ($LASTEXITCODE -ne 0) {
    Write-Host "[AVISO] cargo-sbom nao encontrado. A instalar..." -ForegroundColor Yellow
    cargo install cargo-sbom --locked
    cargo sbom --output-format cyclone_dx_json_1_6 2>$null | Out-File -FilePath $cdx -Encoding UTF8
}
Write-Host "  -> $cdx ($((Get-Item $cdx).Length) bytes)" -ForegroundColor Green

# 2. SPDX 2.3
Write-Host "[2/2] Generating SPDX 2.3 SBOM..." -ForegroundColor Cyan
$spdx = Join-Path $OutDir "rnfc-$tag.spdx.json"
cargo sbom --output-format spdx_json_2_3 2>$null | Out-File -FilePath $spdx -Encoding UTF8
Write-Host "  -> $spdx ($((Get-Item $spdx).Length) bytes)" -ForegroundColor Green

# 3. Summary
Write-Host ""
Write-Host "[OK] SBOMs generated in $OutDir" -ForegroundColor Green
Write-Host "     CycloneDX 1.6: $cdx"
Write-Host "     SPDX 2.3:      $spdx"

# 4. Validate with osv-scanner if available
if (Get-Command osv-scanner -ErrorAction SilentlyContinue) {
    Write-Host ""
    Write-Host "[3/3] Scanning SBOM for vulnerabilities..." -ForegroundColor Cyan
    osv-scanner --sbom=$spdx 2>&1 | Select-Object -First 30
} else {
    Write-Host ""
    Write-Host "[SKIP] osv-scanner not installed (https://osv.dev/)" -ForegroundColor Gray
}
