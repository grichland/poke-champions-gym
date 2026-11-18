# The battle-log proof runs: the rig with rows recorded (on, compared with Showdown) or not (off).
# usage: powershell -File 2026-11-18-sim-log-run.ps1 on|off SEED
param([string]$mode = "on", [int]$seed = 1118)
$ErrorActionPreference = "Continue"
$repo = "C:\Users\user\Code\pokemon_champions\poke-engine"
$res = Join-Path $repo "tools\rig\results"
$log = Join-Path $res "2026-11-18-sim-log-$mode-$seed.log"
Set-Location $repo
"start $(Get-Date -Format o) mode=$mode seed=$seed" | Tee-Object -FilePath $log
$runs = @(@("real", 3000), @("stage5", 2000), @("focus5", 1000), @("stage4", 500), @("stage3", 300), @("stage2", 300), @("plain", 300))
foreach ($r in $runs) {
    $name = $r[0]; $n = $r[1]
    $out = Join-Path $res "2026-11-18-sim-log-$mode-$seed-$name.jsonl.gz"
    $args = @("tools\rig\driver.py", "--sim", "--$name", "--battles", "$n", "--seed", "$seed", "--progress", "500", "--out", $out)
    if ($mode -eq "on") { $args += "--rows" }
    "== $name $n $(Get-Date -Format o)" | Tee-Object -FilePath $log -Append
    & python @args 2>&1 | Tee-Object -FilePath $log -Append
}
foreach ($f in @("stage5_scenarios", "stage5_scenarios2")) {
    "== scenarios $f" | Tee-Object -FilePath $log -Append
    $sargs = @("tools\rig\scenarios.py", "tools\rig\data\$f.json")
    if ($mode -eq "on") { $sargs += "--rows" }
    & python @sargs 2>&1 | Select-Object -Last 3 | Tee-Object -FilePath $log -Append
}
"DONE $(Get-Date -Format o)" | Tee-Object -FilePath $log -Append
