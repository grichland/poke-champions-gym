function H($t){ "`n===== $t =====" }
H 'cargo/rustc/rustup on PATH'
foreach($c in 'cargo','rustc','rustup','maturin','link','cl','gcc','clang','node','npm','git','python','wsl'){ $g=Get-Command $c -ErrorAction SilentlyContinue; "{0}: {1}" -f $c, ($(if($g){$g.Source}else{'-'})) }
H 'dirs'
foreach($d in 'C:\Users\grich\.cargo','C:\Users\grich\.rustup','C:\Users\grich\.rustup\toolchains','C:\Program Files\Rust stable MSVC 1.80','C:\Program Files (x86)\Microsoft Visual Studio','C:\Program Files\Microsoft Visual Studio','C:\Users\grich\Code\pcsa\research\pcsa-rl\poke-engine-eval','C:\Users\grich\Code\pcsa\research\pcsa-rl\poke-engine-eval\_dl','C:\Users\grich\Code\pcsa\research\pcsa-rl\node_modules\pokemon-showdown','C:\Users\grich\Code\pcsa\research\pcsa-rl\.venv'){ "{0}: {1}" -f $d,(Test-Path $d); if((Test-Path $d) -and $d -match 'toolchains|Visual Studio'){ Get-ChildItem $d | Select -Expand Name } }
H 'vswhere'; $vw="${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"; if(Test-Path $vw){& $vw -all -products * -property installationPath}else{'no vswhere'}
H 'versions'
$py='C:\Users\grich\Code\pcsa\research\pcsa-rl\.venv\Scripts\python.exe'; & $py --version; & $py -m pip --version
node --version
H 'pip list'
& $py -m pip list 2>&1 | Select-String -Pattern 'torch|numpy|poke|maturin|pyo3|setuptools|wheel|cffi|gym|stable'
H 'torch cuda'
Get-ChildItem 'C:\Users\grich\Code\pcsa\research\pcsa-rl\.venv\Lib\site-packages' -Directory | Where Name -match '^torch-|^poke' | Select -Expand Name
H 'showdown'
$sd='C:\Users\grich\Code\pcsa\research\pcsa-rl\node_modules\pokemon-showdown'
if(Test-Path $sd){ Get-Content "$sd\package.json" | Select-String '"version"','"name"'; Get-ChildItem $sd -Force | Select -First 5 Name; if(Test-Path "$sd\.git"){'has .git'} }
Get-ChildItem 'C:\Users\grich\Code\pcsa\research\pcsa-rl' -Force -Directory | Select -Expand Name
Get-ChildItem 'C:\Users\grich\Code\pcsa' -Force -Directory | Select -Expand Name
H 'wsl'; wsl --status 2>&1 | Out-String
