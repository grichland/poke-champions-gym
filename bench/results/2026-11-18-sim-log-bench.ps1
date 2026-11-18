# Speed of the battle-log record: HEAD's build against this working tree with recording off and on, in
# alternating order per round (the desktop may be busy), every pool; then the old engine's benchmark.
Set-Location (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$host.UI.RawUI.WindowTitle = "sim-log bench"
$out = "bench/results/2026-11-18-sim-log-desktop.jsonl"
$log = "bench/results/2026-11-18-sim-log-desktop.log"
$builds = @(
    @("HEAD ad935f7 (code of 287e83f)", "target/head-bin/sim_play.exe", ""),
    @("sim-log working tree, rows off", "target/release/sim_play.exe", ""),
    @("sim-log working tree, rows on", "target/release/sim_play.exe", "--log")
)
function Run($label, $exe, $flag, $round, $mode) {
    $args = @("tools/rig/data/teams.json", "50000", "1", "600")
    if ($mode -ne "") { $args += $mode }
    if ($flag -ne "") { $args += $flag }
    $line = & $exe @args
    $tagged = $line -replace '^\{', ('{"build": "' + $label + '", "round": ' + $round + ', ')
    Write-Host $tagged
    [System.IO.File]::AppendAllText((Join-Path (Get-Location) $out), $tagged + "`n")
    [System.IO.File]::AppendAllText((Join-Path (Get-Location) $log), $tagged + "`n")
}
$orders = @(@(0, 1, 2), @(2, 1, 0), @(1, 0, 2))
foreach ($round in 1..3) {
    foreach ($mode in @("", "--stage2", "--stage3", "--stage4", "--stage5", "--real")) {
        foreach ($i in $orders[$round - 1]) { Run $builds[$i][0] $builds[$i][1] $builds[$i][2] $round $mode }
    }
}
foreach ($b in @(@("HEAD ad935f7 (code of 287e83f)", "target/head-bin/random_play.exe"), @("sim-log working tree", "target/release/random_play.exe"))) {
    $rp = (& $b[1] bench/data/preview_states.txt 100000 1000 600) | Where-Object { $_ -like '{*' } | Select-Object -Last 1
    $tagged = $rp -replace '^\{', ('{"build": "' + $b[0] + '", "bench": "random_play (old engine)", ')
    Write-Host $tagged
    [System.IO.File]::AppendAllText((Join-Path (Get-Location) $out), $tagged + "`n")
    [System.IO.File]::AppendAllText((Join-Path (Get-Location) $log), $tagged + "`n")
}
New-Item -ItemType File -Force "bench/results/2026-11-18-sim-log-bench.done" | Out-Null
