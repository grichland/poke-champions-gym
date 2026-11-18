function Sec($t){ "`n===== $t =====" }
$root='C:\Users\grich\Code\pcsa'
Sec 'tools dir'; Get-ChildItem "$root\tools" -Force | Select Name,Mode | Format-Table | Out-String
Sec 'pyvenv.cfg'; Get-Content "$root\research\pcsa-rl\.venv\pyvenv.cfg"
Sec 'site-packages dirs'; Get-ChildItem "$root\research\pcsa-rl\.venv\Lib\site-packages" -Directory | Where Name -match 'dist-info$' | Select -Expand Name
Sec 'node search'; Get-ChildItem $root -Recurse -Depth 3 -Filter node.exe -ErrorAction SilentlyContinue | Select -Expand FullName
Sec 'showdown search'; Get-ChildItem $root -Recurse -Depth 4 -Directory -Filter 'pokemon-showdown*' -ErrorAction SilentlyContinue | Select -Expand FullName
Sec 'wsl status'; $o = (wsl --status 2>&1 | Out-String) -replace "`0",''; $o
Sec 'wsl list'; $o = (wsl -l -v 2>&1 | Out-String) -replace "`0",''; $o
Sec 'git/other toolchain dirs'; Get-ChildItem 'C:\Program Files','C:\Program Files (x86)' -Directory | Select -Expand Name
Sec 'pcsa git'; git -C $root log -1 --format="%h %ad %s" 2>&1
Sec 'rust files anywhere'; Get-ChildItem 'C:\Users\grich' -Depth 1 -Directory -Force -ErrorAction SilentlyContinue | Select -Expand Name
Sec 'll-bench logs tail'; Get-ChildItem "$root\research\pcsa-rl-low-latency\runs\low-latency" -Filter *.log | Sort LastWriteTime | Select -Last 3 Name,LastWriteTime | Format-Table | Out-String
