# Observation speed through Python (pyenv_obs.py, needs numpy) and inside Rust (obs_env --speed), teeing
# to a log beside the results. Use: powershell -File bench\python\run_pyenv_obs.ps1 -Python PATH -Out FILE [-Part python|rust]
param([string]$Python, [string]$Out, [double]$Secs = 30, [string]$Part = "python")
$repo = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$env:PYTHONPATH = Join-Path $repo "python"
$env:PYTHONDONTWRITEBYTECODE = "1"
Set-Location $repo
$log = [IO.Path]::ChangeExtension($Out, ".log")
if ($Part -eq "python") {
    & $Python (Join-Path $repo "bench\python\pyenv_obs.py") (Join-Path $repo "tools\rig\data\teams.json") --secs $Secs --out $Out 2>&1 | Tee-Object -FilePath $log -Append
} else {
    foreach ($cfg in @(@(256, 1), @(4096, 1), @(4096, 8))) {
        cmd /c "target\release\obs_env.exe --speed --n $($cfg[0]) --threads $($cfg[1]) --secs $Secs --rounds 6 --out $Out 2>&1" | Tee-Object -FilePath $log -Append
    }
}
