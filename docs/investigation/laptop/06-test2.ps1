function Sec($t){ "`n===== $t =====" }
$scr='C:\Users\grich\Code\pcsa\poke-engine-investigation'; $d="$scr\exetest2"; New-Item -ItemType Directory -Force $d | Out-Null
$csc="$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
Set-Content "$d\hello.cs" 'class P{static void Main(){System.Console.WriteLine("hello");}}'
Sec 'compile'; & $csc /nologo /out:"$d\hello.exe" "$d\hello.cs"; "exit $LASTEXITCODE"
Sec 'exists right after compile'; Test-Path "$d\hello.exe"; Get-ChildItem $d | Select Name,Length | Format-Table | Out-String
Start-Sleep 2
Sec 'exists after 2s'; Get-ChildItem $d | Select Name,Length | Format-Table | Out-String
Sec 'run'; try{ & "$d\hello.exe"; "exit $LASTEXITCODE"}catch{"FAILED: $_"}
Sec 'exists after run'; Get-ChildItem $d | Select Name,Length | Format-Table | Out-String
Sec 'managed dll'; & $csc /nologo /target:library /out:"$d\lib.dll" "$d\hello.cs" 2>&1 | Select -First 3; Get-ChildItem $d | Select Name,Length | Format-Table | Out-String
Sec 'AV products'; Get-CimInstance -Namespace root\SecurityCenter2 -ClassName AntiVirusProduct | Select displayName,productState | Format-List | Out-String
Sec 'Application/Defender/McAfee events last 10 min'
Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=(Get-Date).AddMinutes(-10)} -ErrorAction SilentlyContinue | Where { $_.ProviderName -match 'McAfee|Defender|SmartScreen' } | Select -First 10 TimeCreated,ProviderName,Message | Format-List | Out-String
Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational'; StartTime=(Get-Date).AddMinutes(-10)} -ErrorAction SilentlyContinue | Where { $_.Message -match 'hello|exetest' } | Select TimeCreated,Id,Message | Format-List | Out-String
Sec 'McAfee processes/services'; Get-Process | Where Name -match 'mc|mfe|mcafee' | Select Name,Id | Format-Table | Out-String
Sec 'Zone/ADS of pyd'; Get-Item "$scr\pydtest\pe_ext\pe_ext.cp312-win_amd64.pyd" -Stream * | Select Stream | Format-Table | Out-String
