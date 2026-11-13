<#
.SYNOPSIS
    Sign an RNFC firmware binary with ECDSA P-256 for STM32 X-CUBE-SBSFU.
.DESCRIPTION
    Takes an unsigned firmware binary, computes SHA-256, signs with the
    private key, and produces a SBSFU-compatible signed image.
.PARAMETER Input
    Path to the unsigned firmware binary (output of cargo build).
.PARAMETER Output
    Path to the signed firmware binary.
.PARAMETER Key
    Path to the ECDSA P-256 private key (PEM format).
.EXAMPLE
    .\sign_firmware.ps1 -Input target\...\rnfc-hal -Output build\rnfc-signed.bin -Key private.pem
#>

param(
    [Parameter(Mandatory=$true)][string]$Input,
    [Parameter(Mandatory=$true)][string]$Output,
    [Parameter(Mandatory=$true)][string]$Key
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Input)) { throw "Input not found: $Input" }
if (-not (Test-Path $Key)) { throw "Key not found: $Key" }

Write-Host "[1/3] Computing SHA-256 of firmware..." -ForegroundColor Cyan
$hash = (Get-FileHash -Path $Input -Algorithm SHA256).Hash.ToLower()
Write-Host "      SHA-256: $hash"

Write-Host "[2/3] Signing hash with ECDSA P-256..." -ForegroundColor Cyan
$tmpHash = [System.IO.Path]::GetTempFileName()
$tmpSig  = [System.IO.Path]::GetTempFileName()
[System.IO.File]::WriteAllBytes($tmpHash, (Get-Content $Input -Raw -AsByteStream))

# openssl dgst -sha256 -sign <key> -out <sig> <hash>
& openssl dgst -sha256 -sign $Key -out $tmpSig $Input
if ($LASTEXITCODE -ne 0) { throw "openssl signing failed" }

Write-Host "[3/3] Building SBSFU-compatible signed image..." -ForegroundColor Cyan

$firmware = [System.IO.File]::ReadAllBytes($Input)
$signature = [System.IO.File]::ReadAllBytes($tmpSig)

# SBSFU image format:
#   [4 bytes: header magic "SBSF"]
#   [4 bytes: firmware length (LE)]
#   [N bytes: firmware payload]
#   [72 bytes: ECDSA P-256 signature (r||s, padded)]
#   [4 bytes: footer magic "1FSB"]

$magic  = [System.Text.Encoding]::ASCII.GetBytes("SBSF")
$footer = [System.Text.Encoding]::ASCII.GetBytes("1FSB")
$lenBytes = [BitConverter]::GetBytes([uint32]$firmware.Length)
if (-not [BitConverter]::IsLittleEndian) { [Array]::Reverse($lenBytes) }

$sigPadded = New-Object byte[] 72
[Array]::Copy($signature, $sigPadded, [Math]::Min($signature.Length, 72))

$out = New-Object System.Collections.Generic.List[byte]
$out.AddRange($magic)
$out.AddRange($lenBytes)
$out.AddRange($firmware)
$out.AddRange($sigPadded)
$out.AddRange($footer)

New-Item -ItemType Directory -Force -Path (Split-Path $Output) | Out-Null
[System.IO.File]::WriteAllBytes($Output, $out.ToArray())

Remove-Item $tmpHash, $tmpSig -Force

Write-Host ""
Write-Host "[OK] Signed image: $Output" -ForegroundColor Green
Write-Host "     Size: $($out.Count) bytes" -ForegroundColor Green
Write-Host "     Firmware: $($firmware.Length) bytes" -ForegroundColor Green
Write-Host "     Signature: 72 bytes (padded)" -ForegroundColor Green

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Flash SBSFU bootloader at 0x08000000"
Write-Host "  2. Flash signed image at 0x08010000"
Write-Host "  3. Reset — bootloader will verify the signature"
