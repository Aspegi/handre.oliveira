# Download the DAF-MIT MagNav dataset (flight data + anomaly maps)
# Source: https://doi.org/10.5281/zenodo.4271803
param([string]$OutDir = "data\magnav")
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$url = "https://raw.githubusercontent.com/MIT-AI-Accelerator/MagNav.jl/main/data/Renfrew_Anomaly_Map.csv"
$out = Join-Path $OutDir "renfrew_anomaly.csv"
Write-Host "[1/1] Downloading Renfrew anomaly map..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing
Write-Host "[OK] $out ($((Get-Item $out).Length) bytes)" -ForegroundColor Green
