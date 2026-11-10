$config = @{ Port=14551; TimeoutSec=400; OutputDir="C:\dev\rnfc"; ImageBytes=9216; SkipPerChunk=2 }
$udp = New-Object System.Net.Sockets.UdpClient $config.Port
$udp.Client.ReceiveTimeout = $config.TimeoutSec * 1000
$frameCount = 0
$chunks = New-Object System.Collections.ArrayList
function Save-Frame($chunks, $config, $frameNum) {
    if ($chunks.Count -eq 0) { return }
    $payload = New-Object System.Collections.Generic.List[byte]
    foreach ($c in $chunks) {
        $cLen = [int]$c[1]
        for ($k = $config.SkipPerChunk; $k -lt $cLen; $k++) { $payload.Add([byte]$c[10+$k]) }
    }
    $pix = $payload.ToArray()
    $final = New-Object byte[] $config.ImageBytes
    [Array]::Copy($pix, 0, $final, 0, [Math]::Min($pix.Length, $config.ImageBytes))
    $hdr = [System.Text.Encoding]::ASCII.GetBytes("P6`n64 48`n255`n")
    $all = New-Object byte[] ($hdr.Length + $final.Length)
    [Array]::Copy($hdr, 0, $all, 0, $hdr.Length)
    [Array]::Copy($final, 0, $all, $hdr.Length, $final.Length)
    $p = Join-Path $config.OutputDir "frame_$frameNum.ppm"
    [System.IO.File]::WriteAllBytes($p, $all)
    Write-Host "[SAVE] frame_$frameNum.ppm ($($payload.Count) bytes)" -ForegroundColor Green
}
Write-Host "A escutar em $($config.Port)..." -ForegroundColor Cyan
$end = $false
while (-not $end) {
    try {
        $ep = New-Object System.Net.IPEndPoint([System.Net.IPAddress]::Any, 0)
        $d = $udp.Receive([ref]$ep)
        if ($d.Length -lt 12 -or $d[0] -ne 253) { continue }
        $msgid = [int]$d[7] -bor ([int]$d[8] -shl 8) -bor ([int]$d[9] -shl 16)
        if ($msgid -eq 130) { if ($chunks.Count -gt 0) { Save-Frame $chunks $config $frameCount; $frameCount++ }; $chunks.Clear(); continue }
        if ($msgid -eq 131) { [void]$chunks.Add($d) }
    } catch { Write-Host "Fim: $($_.Exception.Message)" -ForegroundColor Yellow; $end=$true }
}
if ($chunks.Count -gt 0) { Save-Frame $chunks $config $frameCount; $frameCount++ }
$udp.Close()
Write-Host "Total frames: $frameCount" -ForegroundColor Cyan
