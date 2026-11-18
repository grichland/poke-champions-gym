# Runs the Python tests with the given interpreter (plain-script mode, or pytest when -Pytest), teeing to a log.
param([string]$Python = "C:\Program Files\Python312\python.exe", [string]$Log = "", [switch]$Pytest)
$repo = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$env:PYTHONPATH = Join-Path $repo "python"
$env:PYTHONDONTWRITEBYTECODE = "1"
$tests = @((Join-Path $repo "python\tests\test_env.py"), (Join-Path $repo "python\tests\test_battle_view.py"), (Join-Path $repo "python\tests\test_obs.py"), (Join-Path $repo "python\tests\test_teams_builtin.py"), (Join-Path $repo "python\tests\test_live.py"))
if ($Log -eq "") { $Log = Join-Path $repo "bench\results\pytests.log" }
Set-Location $repo
if ($Pytest) {
    & $Python -m pytest -p no:cacheprovider -v @tests 2>&1 | Tee-Object -FilePath $Log
} else {
    $tests | ForEach-Object { & $Python $_ 2>&1 } | Tee-Object -FilePath $Log
}
