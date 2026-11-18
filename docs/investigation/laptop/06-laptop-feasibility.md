# 06 Laptop feasibility (investigated 16 Nov 2026, ~15:10-15:15 laptop time)

## Summary / verdict
1. ssh works: host AWL, Windows 11 Home build 26200, Core Ultra 9 275HX (24 cores), 32 GB (17.5 GB free at survey), RTX 5070 Laptop 8 GB (0 MiB used), C: 815 GB free. ssh logs in as `awl\pcsa`; our files live under `C:\Users\grich\...` and are writable.
2. Only job running: the `ll-bench` window (PowerShell window + its python parity benchmark, 1.1 GB). It finished at 15:12:35 during the survey. Nothing of ours was touched.
3. NO Rust toolchain on the laptop: no cargo/rustc/rustup/maturin, no MSVC/Build Tools (no vswhere, no cl/link; `Program Files\MSBuild` is only the shared targets), no gcc/clang, no llvm-mingw. `poke-engine-eval` and its `_dl` are NOT on the laptop. So "can we build there" is NO today; it needs the toolchain copied or installed (decision below).
4. Python 3.12.10 venv (uv-made, no pip, no maturin) has torch 2.14.1+cu130, numpy 2.5.4, poke_env 0.16.1, ray, gymnasium, onnxruntime, cffi. Node v24.22.1 and pokemon-showdown 0.11.11 (commit c0845f33, 23 Oct 2026) are at `pcsa\pcsa\.tools\...`, not under research\pcsa-rl\node_modules.
5. Smart App Control is OFF now (`VerifiedAndReputablePolicyState = 0`; `SAC_PreviousState = 1`, `SAC_EnforcementReason = 6`: it was on before and has been turned off). Defender is disabled (`AMServiceEnabled False`); McAfee is the registered AV (mc-fw-host, mc-neo-host running). CodeIntegrity log shows only McAfee/SecurityHealth load errors (3033), nothing about our files.
6. Python loads our kind of .pyd: the desktop-built, unsigned `pe_ext.cp312-win_amd64.pyd` (3.9 MB, gnu target) was scp'd to the laptop and imported by the venv Python: `IMPORT OK`, functions `battle_over, evaluate_state, options, random_battles, step`. No CodeIntegrity event.
7. Unsigned locally compiled .exe: a trivial C# exe built with the in-box csc.exe RAN (`hello`, exit 0) in 6 of 8 attempts. In 2 of the 8 (the first exe of two script runs) the freshly built exe was refused ("Unknown error 0xfffffffe") and the file was DELETED from disk (not a CodeIntegrity event; consistent with McAfee real-time scan removing a first-seen unsigned exe). It is intermittent, so a Rust .exe/.pyd compiled on the laptop may be removed on first run: needs a retry/exclusion strategy.
8. Toolchain sizes on the desktop: `.rustup` 840 MB (only `stable-x86_64-pc-windows-gnu`), `.cargo` 184 MB, `_dl\llvm-mingw` 736 MB (zip 182 MB), `rustup-init.exe` 13 MB. Nothing copied (over the 50 MB limit and a user decision).
9. Visible windows: a scheduled task with an Interactive principal `AWL\grich` (tools\start_visible.ps1, ll_bench_start.ps1) opens a PowerShell window in grich's session; needs grich logged in. Works already (ll-bench window is open now).
10. Not blocked: Python loading unsigned .pyd, running unsigned exe (mostly). At risk: McAfee deleting freshly built binaries. Needs user: how to get the toolchain onto the laptop, McAfee exclusion for the build folder.

## 1. Machine
```
Caption: Microsoft Windows 11 Home  Version 10.0.26200  Build 26200
FreePhysicalMemory 17534356 KB  TotalVisibleMemorySize 32985808 KB
CPU: Intel(R) Core(TM) Ultra 9 275HX  Cores 24  Logical 24   (load 8% at survey)
C: used 1.20 TB, free 815 GB
nvidia-smi: NVIDIA GeForce RTX 5070 Laptop GPU, 0 MiB used, 8151 MiB total, driver 610.88, 0 %
whoami (ssh): awl\pcsa
```
Git is installed (`C:\Program Files\Git\cmd\git.exe`). WSL: `wsl --status` -> "The Windows Subsystem for Linux is not installed. You can install by running 'wsl.exe --install'."

## 2. Running at survey time (15:12)
```
powershell.exe pid 25448 ws 62MB  -NoExit -ExecutionPolicy Bypass -File ...\pcsa-rl-low-latency\configs\laptop\ll_bench_window.ps1 -Jobs "time-ll:...,parts:...,time-base:...,time-belief:...,dmg-live:...,dmg-sim:...,parity:parity:100000:6000"
python.exe pid 5200  ws 4MB   venv launcher: ...\research\pcsa-rl\.venv\Scripts\python.exe -u -m rl2.scripts.ll_bench parity --live ...\ll-data\live --sim ...\runs\exploiter-1116\archive\u000[3-7]*.jsonl.gz --n-live 100000 --n-sim 6000 --out ...\low-latency\parity.json
python.exe pid 36236 ws 1126MB  real interpreter C:\Users\grich\Code\pcsa\tools\python312\python.exe (same args)
```
No node/cargo/rustc. Top memory otherwise: SupportAssistAgent 783 MB, Memory Compression 749, mc-fw-host 609. Scheduled tasks present: ll-bench (was Running), x1116-base/eval/smoke/train, x1116b-train, xcand-eval (Ready), plus Dell/NVIDIA/OneDrive/Edge. parity.log/jobs.log were updated 15:12:35, i.e. the ll-bench batch finished then.

## 3. Toolchain and software present
```
cargo/rustc/rustup/maturin/link/cl/gcc/clang on PATH: none
C:\Users\grich\.cargo, .rustup: absent
Visual Studio / Build Tools / vswhere: absent
...\research\pcsa-rl\poke-engine-eval (and _dl): absent
venv: C:\Users\grich\Code\pcsa\research\pcsa-rl\.venv  (uv venv; home = C:\Users\grich\Code\pcsa\tools\python312, Python 3.12.10; "No module named pip")
dist-info: torch-2.14.1+cu130, numpy-2.5.4, poke_env-0.16.1, ray-2.60.0, gymnasium-1.3.0, onnxruntime-1.31.1, cffi-2.1.1, cryptography, pydantic, setuptools-84.2.1 ... (no maturin, no poke_engine, no pe_ext, no wheel)
node: C:\Users\grich\Code\pcsa\pcsa\.tools\node\node.exe  v24.22.1 (not on PATH)
pokemon-showdown: C:\Users\grich\Code\pcsa\pcsa\.tools\vendor\pokemon-showdown  version 0.11.11, git c0845f33d381402dfcb18ce324c20a94f18af075 (Fri Oct 23 15:15:41 2026)
research\pcsa-rl\node_modules\pokemon-showdown: absent
dotnet csc.exe (in-box .NET Framework 4): present
```
Other `tools` on laptop: python, python-bin, python312, uv, uv-cache, start_visible.ps1, launch_train.ps1, make_venv.ps1.

## How the desktop build was made (evidence from desktop files)
- `poke-engine-eval/build.log`: `maturin develop --release --no-default-features --features ...` (Champions/BSS), poke-engine 0.0.48, pyo3 0.29.0, CPython 3.12, deps include `windows_x86_64_gnu`: the target is `x86_64-pc-windows-gnu`; `C:\Users\user\.rustup\toolchains` holds only `stable-x86_64-pc-windows-gnu` (840 MB). Linker/assembler come from llvm-mingw (`_dl\llvm-mingw\llvm-mingw-20261020-ucrt-x86_64`, 736 MB; rustup-init.exe 13 MB alongside). `build.log` finished in 38 s (release). Cargo registry (`.cargo` 184 MB) was filled by downloading, so crates (pyo3, rand, clap...) are not available offline on the laptop.
- `pe_ext` (cdylib, pyo3 0.29 extension-module, depends on `../poke-engine` with `bss` feature): built output `target\release\pe_ext.dll` 3.98 MB, installed as `pe_ext.cp312-win_amd64.pyd`; `poke_engine.dll` 22.5 MB.
- The exact env vars (PATH to llvm-mingw, CARGO_HOME) were set inside the agent's inline command, not saved in a script; only the log survives.

## 4. Smart App Control / App Control (read-only)
```
Get-MpComputerStatus: SmartAppControlState (empty), AMServiceEnabled False, RealTimeProtectionEnabled False, AntivirusEnabled False
HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy:
  EmodePolicyRequired 0, SkuPolicyRequired 0, VerifiedAndReputablePolicyState 0, SAC_PreviousState 1, SAC_EnforcementReason 6
Win32_DeviceGuard: CodeIntegrityPolicyEnforcementStatus 2, UsermodeCodeIntegrityPolicyEnforcementStatus 0, SecurityServicesRunning {2}
SecurityCenter2 AV: Windows Defender (state 393472), McAfee (state 266240)
AppLocker log: "policy was applied successfully" entries (15 Nov, 11 Nov, 15 Oct, 4 Oct, 24 Sep); no blocks listed.
CodeIntegrity/Operational last 25: Id 3033 errors about mc-fw-host.exe and SecurityHealthService.exe failing to load McAfee/Vulkan/Intel dlls ("did not meet the Custom 3 / Antimalware / Windows signing level"), Id 3089 signature info. None mention our files.
```
Reading: VerifiedAndReputablePolicyState 0 means Smart App Control is off; (User-mode enforcement status 0 = no user-mode CI policy.) Kernel-mode status 2 is the usual "enforced" value for default kernel policy.

## 5. Decisive tests
Scripts: `scripts\06-test.ps1`, `06-test2.ps1`, `06-test3.ps1`; raw outputs `scripts\06-out-test*.txt`. Files left on the laptop under `C:\Users\grich\Code\pcsa\poke-engine-investigation\` (scripts, exetest*, pydtest; about 4 MB).

No Rust toolchain, so the in-box `csc.exe` was used as the nearest proxy for "a locally compiled unsigned exe".

**.pyd test (passed).** `pe_ext.cp312-win_amd64.pyd` (unsigned, `Get-AuthenticodeSignature` -> NotSigned) + `__init__.py` copied by scp to `...\poke-engine-investigation\pydtest\pe_ext`; `python -I -c "sys.path.insert(0,...); import pe_ext"`:
```
IMPORT OK C:\Users\grich\Code\pcsa\poke-engine-investigation\pydtest\pe_ext\__init__.py
['battle_over', 'evaluate_state', 'options', 'pe_ext', 'random_battles', 'step']
py exit: 0
```
Caveat: this .pyd was built on the desktop and copied; a .pyd freshly written by a compiler on the laptop was not tested (no toolchain).

**exe test (intermittent block).**
- test.ps1: csc compiled hello.exe (exit 0), then running it gave `Program 'hello.exe' failed to run: Unknown error (0xfffffffe)` and the file `hello.exe` no longer existed afterwards (hello.cs remained).
- test2.ps1 (new dir): compile ok, exe stayed (3584 bytes), ran `hello`, exit 0; a managed dll built and stayed.
- test3.ps1, four builds in a loop: build 1 failed identically (0xfffffffe) and the exe was deleted; builds 2, 3, 4 ran (`hello 2/3/4`, exit 0) and stayed.
- No CodeIntegrity (3077/3033/3034) event referring to hello.exe or the folders in any run (only McAfee/SecurityHealth noise at the same minute). No Application-log entries from Defender/McAfee/SmartScreen in the last 10 minutes.
- Interpretation: the exe is not blocked by Code Integrity/SAC (those would leave an event and not delete the file); a real-time scanner (McAfee is the active AV) refuses and removes some new unsigned executables at first run. Which ones is not predictable from 8 attempts (2 deleted, 6 ran, always the first exe of a script run in 2 of 3 runs). Not proven to be McAfee: no log line names it. Treat as "new unsigned binaries can vanish on first execution".

## 6. Options if unsigned binaries get blocked
| Option | What it needs | Notes |
|---|---|---|
| Build elsewhere (desktop) and copy the .pyd | Desktop builds are forbidden by the rules for this project (all compute on the laptop) | .pyd copy loads fine today (test above) |
| Retry-on-first-run wrapper (build, run, if file missing rebuild and rerun) | nothing | Cheap workaround; fits the observed pattern |
| McAfee exclusion for the build/output folder (e.g. `C:\Users\grich\Code\pcsa\poke-engine`) | user changes McAfee settings (a system setting, not done here) | Most targeted fix |
| Run everything through the already trusted Python (pyd only, cargo build as pyd; test harness in Python, no standalone .exe) | nothing | The .pyd load was never blocked; cargo build scripts / proc-macros still run .exe's during compile (build.rs, proc-macro host are exes: they could be deleted mid build) |
| Self-signing | a code-signing cert in Trusted Publishers (machine setting) | Only needed if SAC/WDAC were on; they are not |
| WSL2 | not installed; `wsl --install` needs admin/features/download | Not possible under the rules; user decision |
| Turn Smart App Control off | Already off. (Irreversible if it were on) | nothing to do |

## 7. Visible window mechanism (existing scripts)
- `C:\Users\grich\Code\pcsa\tools\start_visible.ps1` (training) and `research\pcsa-rl-low-latency\configs\laptop\ll_bench_start.ps1` (benchmarks; desktop copy at `C:\Users\user\Code\research\pcsa-rl-low-latency\configs\laptop\`): run over ssh as pcsa, they `Register-ScheduledTask` (task name e.g. `ll-bench`, `x1116-train`) with action `powershell.exe -NoExit -ExecutionPolicy Bypass -File <script>` and principal `AWL\grich`, `-LogonType Interactive -RunLevel Limited`, execution time limit zero, allowed on battery, then `Start-ScheduledTask`. The window appears in grich's logged-in desktop session; grich must be logged in. A task already Running (window open) blocks a second start. The window script (`ll_bench_window.ps1`) tees output to a log, uses `Start-Process ... -RedirectStandardOutput/-Error`, and kills the child tree with `taskkill /T /F` after a 600 s cap (the 10-minute benchmark rule).
- `watch_run.ps1` etc. in `research\pcsa-rl` tail console logs in a window (`Get-Content -Wait`).
- Not created or run by this investigation.

## Needs the user's decision
1. Toolchain delivery: nothing may be downloaded here. Options: copy desktop `.rustup\toolchains\stable-x86_64-pc-windows-gnu` (840 MB) + `.cargo` (184 MB, has the crate registry, so builds work offline) + `llvm-mingw` (736 MB) = about 1.76 GB, no installer needed; or install rustup/MSVC Build Tools on the laptop (installers, needs download; forbidden for me). The copy is the lowest-risk path since the same toolchain built pe_ext on the desktop.
2. Whether to add a McAfee exclusion for the build folder (a system setting) or accept a retry wrapper for the intermittent deletion of freshly built unsigned executables.
3. Whether the toolchain copy may be scp'd (about 1.7 GB, over the 50 MB limit I was given), and whether a real Rust hello-world and a fresh pyo3 .pyd build test should follow once it is there.
4. Node/showdown path note: the laptop has Showdown 0.11.11 at `pcsa\pcsa\.tools\vendor\pokemon-showdown`, not under research\pcsa-rl\node_modules; confirm that is the intended location for engine-vs-Showdown comparison.
