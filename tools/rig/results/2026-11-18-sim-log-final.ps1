# The final build's check (after the Imprison, Gravity-balloon and Uproar-Rest row fixes): the stage-5 pools
# with rows on, fresh seed; the full proof runs of seeds 1118 and 1119 are in -run.ps1's outputs.
$repo = "C:\Users\user\Code\pokemon_champions\poke-engine"
Set-Location $repo
$res = "tools\rig\results"
$log = "$res\2026-11-18-sim-log-final-1120.log"
foreach ($r in @(@("real", 3000), @("stage5", 2000), @("focus5", 1000), @("stage4", 500))) {
    "== $($r[0]) $($r[1])" | Tee-Object -FilePath $log -Append
    & python tools\rig\driver.py --sim "--$($r[0])" --rows --battles $r[1] --seed 1120 --out "$res\2026-11-18-sim-log-final-1120-$($r[0]).jsonl.gz" 2>&1 | Tee-Object -FilePath $log -Append
}
foreach ($f in @("stage5_scenarios", "stage5_scenarios2")) {
    & python tools\rig\scenarios.py "tools\rig\data\$f.json" --rows 2>&1 | Select-Object -Last 2 | Tee-Object -FilePath $log -Append
}
"DONE" | Tee-Object -FilePath $log -Append
