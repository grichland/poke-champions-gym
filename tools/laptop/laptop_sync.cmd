@echo off
rem Runs on the laptop: bring the repo to the commit in the bundle copied by sync.sh.
setlocal
set ROOT=C:\Users\grich\Code\pcsa
set REPO=%ROOT%\champions-gym
set BUNDLE=%ROOT%\champions-gym.bundle
if not exist "%REPO%\.git" (
  git clone -q "%BUNDLE%" "%REPO%" || exit /b 1
) else (
  git -C "%REPO%" fetch -q "%BUNDLE%" master || exit /b 1
  rem -f: a result file written here and since committed on the desktop is replaced by the committed copy
  git -C "%REPO%" checkout -q -f -B master FETCH_HEAD || exit /b 1
  git -C "%REPO%" reset -q --hard FETCH_HEAD || exit /b 1
)
git -C "%REPO%" log --oneline -1
