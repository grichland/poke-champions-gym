# One rig run with rows, capped at 10 minutes, its output shown in this window and kept in a log.
# usage: powershell -File 2026-11-18-sim-rowhunt-run1.ps1 TAG POOL SEED N ENGINE_EXE
param([string]$tag, [string]$pool, [int]$seed, [int]$n, [string]$engine)
$repo = "C:\Users\user\Code\pokemon_champions\poke-engine"
$res = Join-Path $repo "tools\rig\results"
$base = Join-Path $res "2026-11-18-sim-rowhunt-$tag-$pool-$seed"
$log = "$base.txt"
Set-Location $repo
$env:RIG_ENGINE = $engine
$a = "tools\rig\driver.py --sim --$pool --rows --battles $n --seed $seed --progress 500 --out `"$base.jsonl.gz`""
$p = Start-Process python -ArgumentList $a -NoNewWindow -PassThru -RedirectStandardOutput $log -RedirectStandardError "$base.err"
$p.PriorityClass = "BelowNormal"
$t0 = Get-Date
$shown = 0
while (-not $p.HasExited) {
    Start-Sleep -Seconds 5
    if (((Get-Date) - $t0).TotalSeconds -gt 600) {
        "TIMEOUT at 600 s: stopping" | Add-Content $log
        & taskkill /T /F /PID $p.Id | Out-Null
        break
    }
    $lines = @(Get-Content "$base.err" -ErrorAction SilentlyContinue)
    if ($lines.Count -gt $shown) { $lines[$shown..($lines.Count - 1)] | ForEach-Object { $_.Substring(0, [Math]::Min(200, $_.Length)) }; $shown = $lines.Count }
}
"DONE $tag $pool $seed $(((Get-Date) - $t0).TotalSeconds) s" | Add-Content $log
"DONE"
