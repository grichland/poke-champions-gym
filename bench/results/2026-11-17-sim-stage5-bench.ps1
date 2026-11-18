# Stage-5 speed: the stage-4 build (target/stage4-build) and this build alternate within each round on the
# plain, stage-2, stage-3 and stage-4 pools; then this build alone on stage-5 and real teams, and the old
# engine's benchmark. One JSON line per run in 2026-11-17-sim-stage5-desktop.jsonl.
Set-Location (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$host.UI.RawUI.WindowTitle = "stage5 bench"
$out = "bench/results/2026-11-17-sim-stage5-desktop.jsonl"
$log = "bench/results/2026-11-17-sim-stage5-desktop.log"
$builds = @(@("stage4 build (HEAD 2c2b590)", "target/stage4-build/sim_play.exe"), @("stage5 working tree", "target/release/sim_play.exe"))
function Run($label, $exe, $round, $mode) {
    $args = @("tools/rig/data/teams.json", "50000", "1", "600")
    if ($mode -ne "") { $args += $mode }
    $line = & $exe @args
    $tagged = $line -replace '^\{', ('{"build": "' + $label + '", "round": ' + $round + ', ')
    Write-Host $tagged
    Add-Content -Path $out -Value $tagged -Encoding utf8
    Add-Content -Path $log -Value $tagged -Encoding utf8
}
foreach ($round in 1..3) {
    foreach ($mode in @("", "--stage2", "--stage3", "--stage4")) {
        $order = if ($round % 2 -eq 1) { @(0, 1) } else { @(1, 0) }
        foreach ($i in $order) { Run $builds[$i][0] $builds[$i][1] $round $mode }
    }
}
foreach ($round in 1..2) {
    foreach ($mode in @("--stage5", "--real")) { Run $builds[1][0] $builds[1][1] $round $mode }
}
$rp = (& ./target/release/random_play.exe bench/data/preview_states.txt 100000 1000 600) | Where-Object { $_ -like '{*' } | Select-Object -Last 1
$tagged = $rp -replace '^\{', '{"build": "stage5 working tree", "bench": "random_play (old engine)", '
Write-Host $tagged
Add-Content -Path $out -Value $tagged -Encoding utf8
Add-Content -Path $log -Value $tagged -Encoding utf8
New-Item -ItemType File -Force "bench/results/2026-11-17-sim-stage5-bench.done" | Out-Null
