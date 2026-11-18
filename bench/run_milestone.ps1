# The full-rules milestone on the real teams: tower vs max_damage (10,000 games, 5,000 per seat
# assignment), tower vs random and max_damage vs random (2,000 each), every check on. Each run is capped at
# 590 s and teed to a log under bench\results.
# Use: powershell -File bench\run_milestone.ps1 [-Threads 24] [-Seed 18] [-Date 2026-11-18]
param([int]$Threads = 24, [int]$Seed = 18, [string]$Date = "2026-11-18")
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
$exe = "target\release\players_run.exe"
foreach ($run in @(@("tower", "max_damage", 10000), @("tower", "random", 2000), @("max_damage", "random", 2000))) {
    $a, $b, $n = $run
    $name = "bench\results\$Date-full-milestone-$a-$b"
    $cmd = "$exe --a $a --b $b --games $n --seed $Seed --threads $Threads --cap 590 --label full-$a-$b " +
        "--out $name.jsonl --games-out $name-games.jsonl --events-out $name-events.jsonl --save-events 20 2>&1"
    Write-Output "running: $cmd"
    cmd /c $cmd | Tee-Object -FilePath "$name.log"
}
Set-Content -Path "bench\results\$Date-full-milestone.done" -Value (Get-Date -Format o)
