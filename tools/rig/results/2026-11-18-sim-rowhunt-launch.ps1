# Starts rig runs in parallel, each in its own window, on a copy of the current rig build.
# usage: powershell -File 2026-11-18-sim-rowhunt-launch.ps1 TAG "pool:seed:n,pool:seed:n,..."
param([string]$tag, [string]$spec)
$repo = "C:\Users\user\Code\pokemon_champions\poke-engine"
$snap = Join-Path $repo "target\rowhunt"
New-Item -ItemType Directory -Force $snap | Out-Null
$exe = Join-Path $snap "rig-$tag.exe"
Copy-Item (Join-Path $repo "target\release\rig.exe") $exe -Force
$runner = Join-Path $repo "tools\rig\results\2026-11-18-sim-rowhunt-run1.ps1"
foreach ($r in $spec.Split(",")) {
    $x = $r.Split(":")
    Start-Process powershell -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$runner`" $tag $($x[0]) $($x[1]) $($x[2]) `"$exe`"" -WindowStyle Minimized
}
"launched $tag"
