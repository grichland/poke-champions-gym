$scr='C:\Users\grich\Code\pcsa\poke-engine-investigation'
$csc="$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
"exetest dir now:"; Get-ChildItem "$scr\exetest" | Select Name,Length | Format-Table | Out-String
for($i=1;$i -le 4;$i++){
  $d="$scr\exetest3_$i"; New-Item -ItemType Directory -Force $d | Out-Null
  Set-Content "$d\hello.cs" "class P{static void Main(){System.Console.WriteLine(`"hello $i`");}}"
  & $csc /nologo /out:"$d\hello.exe" "$d\hello.cs"
  $ex = Test-Path "$d\hello.exe"
  try{ $o = & "$d\hello.exe" 2>&1; "run $i : exists=$ex out=$o exit=$LASTEXITCODE" }catch{ "run $i : exists=$ex FAILED: $_" }
  "  exists after: $(Test-Path "$d\hello.exe")"
}
# same exe run from a fresh process after a pause
Start-Sleep 5
& "$scr\exetest3_1\hello.exe"
Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational'; StartTime=(Get-Date).AddMinutes(-3)} -ErrorAction SilentlyContinue | Where Id -ne 3089 | Select TimeCreated,Id,Message | Format-List | Out-String
