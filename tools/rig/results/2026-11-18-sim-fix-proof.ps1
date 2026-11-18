# Berserk / Anger Shell healing-berry fix and the row fixes: the proof runs, rows on. Each run capped at 10 minutes.
# Usage: -Part A (focus5, stage5) or -Part B (the rest, the seed-94202 rerun, the scenarios).
param([string]$Part = "A", [int]$Seed = 52118)
$repo = "C:\Users\user\Code\pokemon_champions\poke-engine"
Set-Location $repo
$res = "tools\rig\results"
$log = "$res\2026-11-18-sim-fix-proof-$Part-$seed.log"
$seed = $Seed

function Run-Capped([string]$name, [string[]]$argv) {
    "== $name" | Tee-Object -FilePath $log -Append
    $out = "$res\2026-11-18-sim-fix-$name.out.txt"
    $p = Start-Process -FilePath python -ArgumentList $argv -NoNewWindow -PassThru -RedirectStandardOutput $out -RedirectStandardError "$out.err"
    if (-not $p.WaitForExit(600000)) {
        $p.Kill()
        "CAPPED at 10 minutes" | Tee-Object -FilePath $log -Append
    }
    Get-Content $out | Tee-Object -FilePath $log -Append
}

if ($Part -eq "A") {
    foreach ($r in @(@("focus5", 3000), @("stage5", 3000))) {
        Run-Capped "$($r[0])-$seed" @("tools\rig\driver.py", "--sim", "--$($r[0])", "--rows", "--battles", $r[1], "--seed", $seed, "--progress", "500", "--out", "$res\2026-11-18-sim-fix-$($r[0])-$seed.jsonl.gz")
    }
} else {
    foreach ($r in @(@("real", 3000), @("stage4", 500), @("focus4", 500), @("stage3", 300), @("stage2", 300), @("plain", 300))) {
        Run-Capped "$($r[0])-$seed" @("tools\rig\driver.py", "--sim", "--$($r[0])", "--rows", "--battles", $r[1], "--seed", $seed, "--out", "$res\2026-11-18-sim-fix-$($r[0])-$seed.jsonl.gz")
    }
    Run-Capped "focus5-94202-after-$seed" @("tools\rig\driver.py", "--sim", "--focus5", "--rows", "--battles", "800", "--seed", "94202", "--out", "$res\2026-11-18-sim-fix-focus5-94202-after-$seed.jsonl.gz")
    foreach ($f in @("stage5_scenarios", "stage5_scenarios2", "stage5_scenarios3")) {
        Run-Capped "scen-$f-$seed" @("tools\rig\scenarios.py", "tools\rig\data\$f.json", "--rows")
    }
}
"DONE" | Tee-Object -FilePath $log -Append
