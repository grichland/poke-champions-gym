# Stage-5 rig runs: one window per group, each run tee'd to its own log; a .done file marks the end of a group.
# usage: powershell -File tools/rig/results/2026-11-17-sim-stage5-run.ps1 -Group <name> -Runs "mode:battles:seed,..." -Tag <tag>
param([string]$Group, [string]$Runs, [string]$Tag = "final")
Set-Location (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)))
$host.UI.RawUI.WindowTitle = "stage5 rig: $Group"
foreach ($r in $Runs.Split(",")) {
    $mode, $n, $seed = $r.Split(":")
    $base = "tools/rig/results/2026-11-17-sim-stage5-$Tag-$mode-seed$seed"
    Write-Host "== $mode $n battles seed $seed -> $base"
    python tools/rig/driver.py --sim "--$mode" --battles $n --seed $seed --progress 250 --out "$base.jsonl.gz" 2>&1 | Tee-Object -FilePath "$base.log"
}
New-Item -ItemType File -Force "tools/rig/results/2026-11-17-sim-stage5-$Tag-$Group.done" | Out-Null
