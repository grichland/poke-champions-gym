$ErrorActionPreference='Continue'
function H($t){ "`n===== $t =====" }
H 'host/os'; hostname; (Get-CimInstance Win32_OperatingSystem | Select Caption,Version,BuildNumber,FreePhysicalMemory,TotalVisibleMemorySize | Format-List | Out-String)
H 'cpu'; (Get-CimInstance Win32_Processor | Select Name,NumberOfCores,NumberOfLogicalProcessors | Format-List | Out-String)
H 'disk'; (Get-PSDrive C | Select Used,Free | Format-List | Out-String)
H 'gpu'; $n="$env:SystemRoot\System32\nvidia-smi.exe"; if(Test-Path $n){& $n --query-gpu=name,memory.used,memory.total,driver_version,utilization.gpu --format=csv} else {'no nvidia-smi in System32'}
H 'processes (python/node/cargo/rustc/etc)'
Get-CimInstance Win32_Process | Where-Object { $_.Name -match 'python|node|cargo|rustc|rustup|maturin|pwsh|powershell|ollama|java|showdown' } | ForEach-Object { "{0} pid={1} ws={2}MB`n   {3}" -f $_.Name,$_.ProcessId,[int]($_.WorkingSetSize/1MB),$_.CommandLine }
H 'top mem'; Get-Process | Sort WS -desc | Select -First 8 Name,Id,@{n='MB';e={[int]($_.WS/1MB)}} | Format-Table | Out-String
H 'cpu load'; (Get-CimInstance Win32_Processor).LoadPercentage
H 'whoami'; whoami; Get-Date
H 'scheduled tasks'; Get-ScheduledTask | Where-Object {$_.TaskPath -eq '\'} | Select TaskName,State | Format-Table | Out-String
