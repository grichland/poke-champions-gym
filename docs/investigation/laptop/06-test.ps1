function Sec($t){ "`n===== $t =====" }
$root='C:\Users\grich\Code\pcsa'; $scr="$root\poke-engine-investigation"
$py="$root\research\pcsa-rl\.venv\Scripts\python.exe"
Sec 'node'; & "$root\pcsa\.tools\node\node.exe" --version
Sec 'showdown'; $sd="$root\pcsa\.tools\vendor\pokemon-showdown"; (Get-Content "$sd\package.json" -TotalCount 12) -join "`n"; if(Test-Path "$sd\.git"){ git -C $sd log -1 --format="%H %ad" 2>&1 } else {'no .git'}
Get-ChildItem "$root\pcsa\.tools\vendor" -Name; Get-ChildItem "$root\pcsa\.tools" -Name
Sec 'start_visible.ps1'; Get-Content "$root\tools\start_visible.ps1"
Sec 'dotnet csc'; $csc="$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"; Test-Path $csc
Sec 'compile+run unsigned C# exe'
$d="$scr\exetest"; New-Item -ItemType Directory -Force $d | Out-Null
Set-Content "$d\hello.cs" 'class P{static void Main(){System.Console.WriteLine("hello from locally compiled exe");}}'
if(Test-Path $csc){ & $csc /nologo /out:"$d\hello.exe" "$d\hello.cs"; "compile exit: $LASTEXITCODE"; try { & "$d\hello.exe"; "run exit: $LASTEXITCODE" } catch { "RUN FAILED: $_" } }
Sec 'copy of an unsigned native exe? (python-built none) signature of the new exe'; Get-AuthenticodeSignature "$d\hello.exe" | Select Status | Format-List | Out-String
Sec 'pyd proxy test'
$pyd="$scr\pydtest\pe_ext"; if(Test-Path "$pyd\pe_ext.cp312-win_amd64.pyd"){
  Get-AuthenticodeSignature "$pyd\pe_ext.cp312-win_amd64.pyd" | Select Status | Format-List | Out-String
  $env:PYTHONDONTWRITEBYTECODE='1'
  & $py -I -c "import sys; sys.path.insert(0, r'$scr\pydtest'); import pe_ext; print('IMPORT OK', pe_ext.__file__); print([n for n in dir(pe_ext) if not n.startswith('_')][:20])" 2>&1
  "py exit: $LASTEXITCODE"
} else {'pyd not copied'}
Sec 'CodeIntegrity events since start'; Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational'; StartTime=(Get-Date).AddMinutes(-3)} -ErrorAction SilentlyContinue | Where Id -ne 3089 | Select TimeCreated,Id,Message | Format-List | Out-String
Sec 'cleanup note'; "files left in $scr (exetest, pydtest)"
