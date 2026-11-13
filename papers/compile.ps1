<#
.SYNOPSIS
    Compile both papers (SCC and CSR) to PDF.
.DESCRIPTION
    Runs pdflatex + bibtex + pdflatex twice for each paper.
    Requires MiKTeX (Windows) or TeX Live (Linux/Mac).
#>

$ErrorActionPreference = "Stop"

# Check pdflatex
if (-not (Get-Command pdflatex -ErrorAction SilentlyContinue)) {
    Write-Host "[ERRO] pdflatex nao encontrado." -ForegroundColor Red
    Write-Host "  Windows: instala MiKTeX de https://miktex.org/download" -ForegroundColor Yellow
    Write-Host "  Linux:   sudo apt install texlive-full" -ForegroundColor Yellow
    Write-Host "  Mac:     brew install --cask mactex" -ForegroundColor Yellow
    exit 1
}

function Compile-Paper($dir) {
    Write-Host "`n=== Compilando $dir ===" -ForegroundColor Cyan
    Push-Location $dir
    try {
        Write-Host "[1/4] pdflatex (pass 1)..." -ForegroundColor Gray
        pdflatex -interaction=nonstopmode main.tex | Out-Null
        Write-Host "[2/4] bibtex..." -ForegroundColor Gray
        bibtex main | Out-Null
        Write-Host "[3/4] pdflatex (pass 2)..." -ForegroundColor Gray
        pdflatex -interaction=nonstopmode main.tex | Out-Null
        Write-Host "[4/4] pdflatex (pass 3)..." -ForegroundColor Gray
        pdflatex -interaction=nonstopmode main.tex | Out-Null

        if (Test-Path "main.pdf") {
            $size = (Get-Item "main.pdf").Length
            Write-Host "[OK] $dir/main.pdf ($size bytes)" -ForegroundColor Green
        } else {
            Write-Host "[FALHA] $dir/main.pdf nao gerado" -ForegroundColor Red
        }
    } finally {
        Pop-Location
    }
}

Compile-Paper "papers\scc"
Compile-Paper "papers\csr"

Write-Host "`n=== Resultado ===" -ForegroundColor Cyan
Get-ChildItem "papers\scc\main.pdf", "papers\csr\main.pdf" -ErrorAction SilentlyContinue | Format-Table FullName, Length

Write-Host "`nSe houver erros, verifica:" -ForegroundColor Yellow
Write-Host "  - papers/scc/main.log e papers/csr/main.log"
Write-Host "  - Citações em falta (bibtex warnings)"
Write-Host "  - Pacotes LaTeX em falta (MiKTeX instala automaticamente)"
