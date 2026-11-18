@echo off
rem Unpack the three toolchain archives copied by push_toolchain.ps1. Run on the laptop.
setlocal
set ROOT=C:\Users\grich\Code\pcsa\tools\rust
for %%N in (rustup cargo llvm-mingw) do (
  if exist "%ROOT%\%%N.tar" (
    if exist "%ROOT%\%%N" rmdir /s /q "%ROOT%\%%N"
    mkdir "%ROOT%\%%N"
    echo unpacking %%N
    tar -xf "%ROOT%\%%N.tar" -C "%ROOT%\%%N"
    if errorlevel 1 (echo FAILED %%N & exit /b 1)
    del "%ROOT%\%%N.tar"
  )
)
set RUSTUP_HOME=%ROOT%\rustup
set CARGO_HOME=%ROOT%\cargo
"%ROOT%\cargo\bin\cargo.exe" --version
"%ROOT%\cargo\bin\rustc.exe" --version
dir /b "%ROOT%"
