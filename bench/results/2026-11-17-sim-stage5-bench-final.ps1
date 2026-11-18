# Two more alternating rounds with the final stage-5 build (after the Future Sight Life Orb, gem and Smack Down
# fixes), appended to 2026-11-17-sim-stage5-desktop.jsonl as rounds 4 and 5.
Set-Location (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
$host.UI.RawUI.WindowTitle = "stage5 bench (final build)"
$out = "bench/results/2026-11-17-sim-stage5-desktop.jsonl"
$log = "bench/results/2026-11-17-sim-stage5-desktop.log"
$builds = @(@("stage4 build (HEAD 2c2b590)", "target/stage4-build/sim_play.exe"), @("stage5 final build", "target/release/sim_play.exe"))
function Run($label, $exe, $round, $mode) {
    $args = @("tools/rig/data/teams.json", "50000", "1", "600")
    if ($mode -ne "") { $args += $mode }
    $line = & $exe @args
    $tagged = $line -replace '^\{', ('{"build": "' + $label + '", "round": ' + $round + ', ')
    Write-Host $tagged
    [System.IO.File]::AppendAllText((Join-Path (Get-Location) $out), $tagged + "`n")
    [System.IO.File]::AppendAllText((Join-Path (Get-Location) $log), $tagged + "`n")
}
foreach ($round in 4..5) {
    foreach ($mode in @("", "--stage2", "--stage3", "--stage4")) {
        $order = if ($round % 2 -eq 1) { @(0, 1) } else { @(1, 0) }
        foreach ($i in $order) { Run $builds[$i][0] $builds[$i][1] $round $mode }
    }
    foreach ($mode in @("--stage5", "--real")) { Run $builds[1][0] $builds[1][1] $round $mode }
}
New-Item -ItemType File -Force "bench/results/2026-11-17-sim-stage5-bench-final.done" | Out-Null
