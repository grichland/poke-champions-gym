# Runs pyenv_play.py with the given interpreter (needs numpy), teeing to a log next to the results.
param([string]$Python, [string]$Out, [double]$Secs = 30)
$repo = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$env:PYTHONPATH = Join-Path $repo "python"
$env:PYTHONDONTWRITEBYTECODE = "1"
Set-Location $repo
$log = [IO.Path]::ChangeExtension($Out, ".log")
& $Python (Join-Path $repo "bench\python\pyenv_play.py") (Join-Path $repo "tools\rig\data\teams.json") --secs $Secs --out $Out 2>&1 | Tee-Object -FilePath $log
