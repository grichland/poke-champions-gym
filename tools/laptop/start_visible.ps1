# Runs on the laptop over ssh: opens a visible PowerShell window on the logged-in desktop that runs one
# repo script under Git Bash and writes its output to a log. Needs grich logged in. One job at a time.
param([Parameter(Mandatory)][string]$Script, [string]$ScriptArgs = '', [Parameter(Mandatory)][string]$Log,
      [string]$Name = 'champions-gym')
$repo = 'C:\Users\grich\Code\pcsa\champions-gym'
$old = Get-ScheduledTask -TaskName $Name -ErrorAction SilentlyContinue
if ($old -and $old.State -eq 'Running') { throw "task $Name is still running: close its window first" }
$bash = 'C:\Program Files\Git\bin\bash.exe'
$cmd = "`$host.UI.RawUI.WindowTitle = '$Name'; & '$bash' -lc 'cd /c/Users/grich/Code/pcsa/champions-gym && bash $Script $ScriptArgs' 2>&1 | Tee-Object -FilePath '$Log'"
$arg = "-NoExit -ExecutionPolicy Bypass -Command `"$cmd`""
$a = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arg -WorkingDirectory $repo
$p = New-ScheduledTaskPrincipal -UserId 'AWL\grich' -LogonType Interactive -RunLevel Limited
$s = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::FromMinutes(15)) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $Name -Action $a -Principal $p -Settings $s -Force | Out-Null
Start-ScheduledTask -TaskName $Name
Start-Sleep 2
Get-ScheduledTask -TaskName $Name | Select-Object TaskName, State
