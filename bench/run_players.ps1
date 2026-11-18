# One players_run in its own window, output teed to a log beside the results.
# Use: powershell -NoExit -File bench\run_players.ps1 -A tower -B max_damage -Games 10000 [-Seed 17] [-Threads 1]
param(
    [string]$A = "tower",
    [string]$B = "max_damage",
    [int]$Games = 1000,
    [int]$Seed = 17,
    [int]$Threads = 1,
    [string]$Date = "2026-11-17",
    [string]$Tag = "",
    [string]$Extra = ""
)
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
$name = "bench\results\$Date-players-run-$A-$B$Tag"
$exe = "target\release\players_run.exe"
$cmd = "$exe --a $A --b $B --games $Games --seed $Seed --threads $Threads --cap 600 --label $A-$B " +
    "--out $name.jsonl --games-out $name-games.jsonl --events-out $name-events.jsonl --save-events 50 $Extra 2>&1"
Write-Output "running: $cmd"
cmd /c $cmd | Tee-Object -FilePath "$name.log"
Write-Output "done: $name.log"
