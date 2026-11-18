# The observation checks inside the environment (two 2,000-game runs) in their own window, teed to a log.
# Use: powershell -NoExit -File bench\run_obs_env.ps1 [-Games 2000] [-Date 2026-11-18]
param([int]$Games = 2000, [int]$Seed = 1, [string]$Date = "2026-11-18")
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
$exe = "target\release\obs_env.exe"
$log = "bench\results\$Date-obs-env-check.log"
foreach ($pair in @(@("tower", "max_damage"), @("random", "random"))) {
    $a = $pair[0]; $b = $pair[1]
    $name = "bench\results\$Date-obs-env-check-$a-$b"
    cmd /c "$exe --a $a --b $b --games $Games --seed $Seed --n 64 --out $name.jsonl --games-out $name-games.jsonl 2>&1" | Tee-Object -FilePath $log -Append
}
Write-Output "done: $log"
