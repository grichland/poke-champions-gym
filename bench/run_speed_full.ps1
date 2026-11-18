# Speed on the milestone's setup (real teams, every rule), teed to logs under bench\results:
#  1. players_run tower vs max_damage, checks off, one thread and all cores, in alternating rounds;
#  2. obs_env --speed: one environment, with and without observations at every decision (alternating
#     inside the run), N = 256 and 4096, observations on 1 thread and on all cores;
#  3. Python (numpy interpreter): pyenv_obs.py, N = 256 and 4096, the same modes.
# Each run is capped well under 10 minutes.
# Use: powershell -File bench\run_speed_full.ps1 -Python PATH [-Cores 24] [-Date 2026-11-18]
param([string]$Python, [int]$Cores = 24, [string]$Date = "2026-11-18")
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
$out = "bench\results\$Date-full-speed"
function Load { (Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average }
"start $(Get-Date -Format o) cpu load $(Load) %" | Tee-Object -FilePath "$out.log" -Append
foreach ($round in 1..2) {
    foreach ($t in @(1, $Cores)) {
        $cmd = "target\release\players_run.exe --a tower --b max_damage --games 2000000 --seed $round --threads $t --cap 20 --no-checks --label speed-r$round-t$t --out $out-players.jsonl 2>&1"
        "round $round threads $t cpu load $(Load) %: $cmd" | Tee-Object -FilePath "$out.log" -Append
        cmd /c $cmd | Tee-Object -FilePath "$out.log" -Append
    }
}
foreach ($cfg in @(@(256, 1), @(4096, 1), @(256, $Cores), @(4096, $Cores))) {
    "obs_env n $($cfg[0]) threads $($cfg[1]) cpu load $(Load) %" | Tee-Object -FilePath "$out.log" -Append
    cmd /c "target\release\obs_env.exe --speed --n $($cfg[0]) --threads $($cfg[1]) --secs 24 --rounds 6 --out $out-rust.jsonl 2>&1" | Tee-Object -FilePath "$out.log" -Append
}
$env:PYTHONPATH = Join-Path $repo "python"
$env:PYTHONDONTWRITEBYTECODE = "1"
"python cpu load $(Load) %" | Tee-Object -FilePath "$out.log" -Append
& $Python "bench\python\pyenv_obs.py" "tools\rig\data\teams.json" --secs 24 --rounds 6 --games "256,4096" --threads "1,$Cores" --out "$out-python.jsonl" 2>&1 | Tee-Object -FilePath "$out.log" -Append
"end $(Get-Date -Format o) cpu load $(Load) %" | Tee-Object -FilePath "$out.log" -Append
Set-Content -Path "$out.done" -Value (Get-Date -Format o)
