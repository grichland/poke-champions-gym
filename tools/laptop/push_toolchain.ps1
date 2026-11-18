# Copy the Rust toolchain from this desktop to the laptop (no installer, nothing downloaded there).
# Run on the desktop. Writes a log next to this script. Safe to rerun: it replaces the laptop copy.
$ErrorActionPreference = 'Stop'
$log = Join-Path $PSScriptRoot 'push_toolchain.log'
Start-Transcript -Path $log -Force | Out-Null

$key = "$env:USERPROFILE\.ssh\id_ed25519"
$dest = 'USER@LAPTOP_HOST'
$sshOpts = @('-i', $key, '-o', 'IdentitiesOnly=yes', '-o', 'BatchMode=yes')
$remoteRoot = 'C:/Users/grich/Code/pcsa/tools/rust'
$llvm = if ($env:LLVM_MINGW) { $env:LLVM_MINGW } else {
    'C:\Users\user\Code\pokemon_champions\research\pcsa-rl\poke-engine-eval\_dl\llvm-mingw\llvm-mingw-20261020-ucrt-x86_64'
}
$stage = Join-Path $env:TEMP 'champions-gym-toolchain'
New-Item -ItemType Directory -Force $stage | Out-Null

$parts = @(
    @{ name = 'rustup'; dir = "$env:USERPROFILE\.rustup" },
    @{ name = 'cargo'; dir = "$env:USERPROFILE\.cargo" },
    @{ name = 'llvm-mingw'; dir = $llvm }
)

& ssh @sshOpts $dest "if not exist `"$($remoteRoot -replace '/', '\')`" mkdir `"$($remoteRoot -replace '/', '\')`""
foreach ($p in $parts) {
    $tar = Join-Path $stage "$($p.name).tar"
    Write-Host "packing $($p.name) from $($p.dir)"
    & tar -cf $tar -C $p.dir .
    if ($LASTEXITCODE -ne 0) { throw "tar failed for $($p.name)" }
    Write-Host ("  {0:N0} MB" -f ((Get-Item $tar).Length / 1MB))
    Write-Host "copying $($p.name)"
    & scp @sshOpts $tar "${dest}:$remoteRoot/$($p.name).tar"
    if ($LASTEXITCODE -ne 0) { throw "scp failed for $($p.name)" }
    Remove-Item $tar
}
# unpack and check with a script file on the laptop: inline commands over ssh break on quoting
Write-Host 'unpacking and checking on the laptop'
& scp @sshOpts (Join-Path $PSScriptRoot 'unpack_toolchain.cmd') "${dest}:$remoteRoot/unpack_toolchain.cmd"
& ssh @sshOpts $dest 'C:\Users\grich\Code\pcsa\tools\rust\unpack_toolchain.cmd'
if ($LASTEXITCODE -ne 0) { throw 'unpack failed' }
Write-Host 'done'
Stop-Transcript | Out-Null
