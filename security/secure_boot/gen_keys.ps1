<#
.SYNOPSIS
    Generate ECDSA P-256 key pair for STM32 X-CUBE-SBSFU.
.DESCRIPTION
    Creates private.pem (keep offline, never commit) and public.pem
    (embedded in SBSFU bootloader).
#>

param(
    [string]$OutDir = "security\secure_boot"
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command openssl -ErrorAction SilentlyContinue)) {
    throw "openssl not found. Install from https://slproweb.com/products/Win32OpenSSL.html"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$priv = Join-Path $OutDir "private.pem"
$pub  = Join-Path $OutDir "public.pem"

if (Test-Path $priv) {
    Write-Host "[AVISO] $priv ja existe — nao vou sobrescrever" -ForegroundColor Yellow
    exit 1
}

Write-Host "[1/3] Generating ECDSA P-256 private key..." -ForegroundColor Cyan
& openssl ecparam -name prime256v1 -genkey -noout -out $priv
if ($LASTEXITCODE -ne 0) { throw "key generation failed" }

Write-Host "[2/3] Deriving public key..." -ForegroundColor Cyan
& openssl ec -in $priv -pubout -out $pub
if ($LASTEXITCODE -ne 0) { throw "public key derivation failed" }

Write-Host "[3/3] Setting permissions..." -ForegroundColor Cyan
icacls $priv /inheritance:r /grant:r "$($env:USERNAME):(R,W)" | Out-Null

Write-Host ""
Write-Host "[OK] Keys generated" -ForegroundColor Green
Write-Host "     Private: $priv" -ForegroundColor Yellow
Write-Host "              (KEEP OFFLINE — never commit)" -ForegroundColor Yellow
Write-Host "     Public:  $pub" -ForegroundColor Green
Write-Host "              (embed in SBSFU bootloader)" -ForegroundColor Green

Write-Host ""
Write-Host "IMPORTANT:" -ForegroundColor Red
Write-Host "  - Backup private.pem to an air-gapped machine or HSM"
Write-Host "  - If lost, you cannot update firmware in the field"
Write-Host "  - If stolen, attacker can sign malicious firmware"

$hash = (Get-FileHash $priv -Algorithm SHA256).Hash
Write-Host ""
Write-Host "Private key SHA-256: $hash" -ForegroundColor Gray
