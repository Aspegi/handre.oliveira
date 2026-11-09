# Teste de matriz de vento para o RNFC (v2 - corrigido)
$ErrorActionPreference = "Continue"
$mainPath = "C:\dev\rnfc\crates\rnfc-backend-sitl\src\main.rs"

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " TESTE 1 - MATRIZ DE VENTO (6 CENARIOS)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$backup = $mainPath + ".bak"
Copy-Item $mainPath $backup -Force
Write-Host "Backup criado: $backup" -ForegroundColor Gray

# Valores como STRINGS (para preservar o .0)
$scenarios = @(
    @("0.0",  "0.0",  "0 m/s (sem vento)"),
    @("2.0",  "-1.0", "2 m/s (leve)"),
    @("4.0",  "-2.0", "4 m/s (moderado)"),
    @("6.0",  "-3.0", "6 m/s (forte)"),
    @("8.0",  "-4.0", "8 m/s (muito forte)"),
    @("12.0", "-6.0", "12 m/s (extremo)")
)

$results = @()

foreach ($scenario in $scenarios) {
    $windN = $scenario[0]
    $windE = $scenario[1]
    $label = $scenario[2]

    Write-Host "`n========================================" -ForegroundColor Yellow
    Write-Host " TESTE: $label (WIND_N=$windN, WIND_E=$windE)" -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Yellow

    $content = Get-Content $mainPath -Raw
    $content = $content -replace 'const WIND_N: f32 = [0-9\.\-]+;', "const WIND_N: f32 = $windN;"
    $content = $content -replace 'const WIND_E: f32 = [0-9\.\-]+;', "const WIND_E: f32 = $windE;"
    [System.IO.File]::WriteAllText($mainPath, $content, (New-Object System.Text.UTF8Encoding $false))

    Write-Host "  A compilar..." -ForegroundColor Gray
    $buildOutput = cargo build -p rnfc-backend-sitl --release 2>&1
    if ($buildOutput -match "error\[") {
        Write-Host "  ERRO DE COMPILACAO" -ForegroundColor Red
        $buildOutput | Select-String "error\[" | Select-Object -First 2
        $results += [PSCustomObject]@{ Vento = $label; Tempo = "ERRO"; Precisao = "ERRO"; Abortou = "ERRO" }
        continue
    }

    Write-Host "  A correr..." -ForegroundColor Gray
    $runOutput = cargo run -p rnfc-backend-sitl --release 2>&1 | Select-String "MISSAO COMPLETA|ABORTAR|distancia a base" | Select-Object -Last 5

    $tempo = "N/A"
    $precisao = "N/A"
    $abortou = "Nao"

    foreach ($line in $runOutput) {
        if ($line -match "MISSAO COMPLETA em t=([0-9\.]+)s") { $tempo = "$($matches[1])s" }
        if ($line -match "distancia a base: ([0-9\.]+)m") { $precisao = "$($matches[1])m" }
        if ($line -match "HOVER:.*a pairar") { $abortou = "HOVER" }
        if ($line -match "ABORTAR") { $abortou = "SIM" }
        if ($line -match "HOVER:.*a pairar") { $abortou = "HOVER"; if ($tempo -eq "N/A") { $tempo = "~180s" } }
    }

    if ($tempo -eq "N/A" -and $abortou -eq "Nao") { $tempo = ">600s" }

    Write-Host "  Resultado: tempo=$tempo precisao=$precisao abortou=$abortou" -ForegroundColor Green
    $results += [PSCustomObject]@{ Vento = $label; Tempo = $tempo; Precisao = $precisao; Abortou = $abortou }
}

Copy-Item $backup $mainPath -Force
Remove-Item $backup -Force
Write-Host "`n main.rs restaurado ao original" -ForegroundColor Gray

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host " RESULTADOS DA MATRIZ DE VENTO" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
$results | Format-Table -AutoSize

Write-Host "`n=== ANALISE ===" -ForegroundColor Cyan
$completos = ($results | Where-Object { $_.Abortou -eq "Nao" -and $_.Tempo -ne "N/A" -and $_.Tempo -ne ">600s" -and $_.Tempo -ne "ERRO" }).Count
$abortados = ($results | Where-Object { $_.Abortou -eq "SIM" }).Count
$erros = ($results | Where-Object { $_.Tempo -eq "ERRO" }).Count

Write-Host "Missoes completas: $completos / 6"
Write-Host "Abortadas: $abortados / 6"
Write-Host "Erros: $erros / 6"