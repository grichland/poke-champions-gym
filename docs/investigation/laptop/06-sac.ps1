function H($t){ "`n===== $t =====" }
H 'MpComputerStatus'; try{ Get-MpComputerStatus | Select SmartAppControlState,AMServiceEnabled,RealTimeProtectionEnabled,AntivirusEnabled | Format-List | Out-String }catch{ "err: $_" }
H 'CI policy reg'; try{ Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy' | Format-List | Out-String }catch{ "err: $_" }
H 'WDAC / DeviceGuard'; try{ Get-CimInstance -ClassName Win32_DeviceGuard -Namespace root\Microsoft\Windows\DeviceGuard | Select CodeIntegrityPolicyEnforcementStatus,UsermodeCodeIntegrityPolicyEnforcementStatus,SecurityServicesRunning | Format-List | Out-String }catch{ "err: $_" }
H 'CodeIntegrity events'; try{ Get-WinEvent -LogName 'Microsoft-Windows-CodeIntegrity/Operational' -MaxEvents 25 -ErrorAction Stop | Select TimeCreated,Id,LevelDisplayName,@{n='M';e={($_.Message -split "`n")[0]}} | Format-Table -Wrap | Out-String -Width 300 }catch{ "err: $_" }
H 'AppLocker events'; try{ Get-WinEvent -LogName 'Microsoft-Windows-AppLocker/EXE and DLL' -MaxEvents 5 -ErrorAction Stop | Format-List TimeCreated,Message | Out-String }catch{ "err: $_" }
