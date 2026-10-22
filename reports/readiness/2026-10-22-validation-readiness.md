# Validation readiness: browser and Tauri validation path, concurrency limits

Date: 2026-10-22, 16:54-17:02 local. Worker session `9b0e8fc9-8828-4ef3-8d63-ab3d05dc733d` (validation
readiness). Numbers are pasted from the tools that produced them (ai/RULES.md rule 7). Machine: macOS (darwin 25), 10 cores, 64 GB RAM.
Scratch tree: `/tmp/obs-readiness/leptos-tauri` (Cargo workspace: `ui` = Leptos 0.8.22 CSR built by
Trunk 0.22.0 from `ui/Trunk.toml` with `[serve] port = 3141`; `src-tauri` = Tauri 2.12.5 shell with
`devUrl http://localhost:3141` and `beforeDevCommand trunk serve --config ui/Trunk.toml`). No change to
the tauri.conf.json was needed: it already pointed at 3141.

Tool versions pasted: `trunk 0.22.0`, `tauri-cli 2.12.4`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`,
`agent-browser 0.37.1` (Google Chrome 155.0.8059.85 used as the engine), `cargo-llvm-cov 0.9.1`.
Environment quirk confirmed: `CARGO_TARGET_DIR=/Users/dct/.cargo/shared_rust_target` is set in the
environment (not in `~/.cargo/config.toml`, which only sets `rustc-wrapper = "sccache"` and
`[profile.dev] opt-level = 1, debug = 1`). Baseline load before the run: `load averages: 3,05 6,26 9,34`.
The repository at `/Users/dct/code/EllyaOrg/projects/dalimaray/droid-observatory` was not touched
(`git status --short` after the run shows only the pre-existing session-log change and build-log files).

Verdict: the browser path is executable and automatable end to end today. The Tauri path starts,
is observable as a process and via System Events, but visual evidence (screenshot) could not be
verified in this run because the screen was locked (see Gotchas); agent-browser cannot attach to it.

## Browser path

Command (background, log to file):

```sh
cd /tmp/obs-readiness/leptos-tauri
trunk serve --config ui/Trunk.toml --port 3141 --open false > /tmp/obs-readiness/trunk-serve.log 2>&1
```

Time until `curl -sf http://localhost:3141/` answered: **55.94 s** (poll 45 of a 1 s loop). The
first dev-profile wasm build was cold (the prior worker only built `--release`), cargo reported
`Finished dev profile [optimized + debuginfo] target(s) in 53.64s`, 190 `Compiling` lines.
Incremental rebuilds after touching `ui/src/main.rs`: `2.08s` and `1.46s`.

Log excerpt:

```text
2026-10-22T14:54:47.240948Z  INFO Starting trunk 0.22.0
2026-10-22T14:54:47.249781Z  INFO starting build
    Finished `dev` profile [optimized + debuginfo] target(s) in 53.64s
2026-10-22T14:55:42.588048Z  INFO success
2026-10-22T14:55:42.588975Z  INFO     http://127.0.0.1:3141/
2026-10-22T14:55:42.588979Z  INFO     http://[::1]:3141/
```

agent-browser against it (all succeeded, first-run, no download: Chrome at `/Applications/Google Chrome.app`):

```text
$ time agent-browser open http://localhost:3141
[agent-browser] launched browser
  readiness
  http://localhost:3141/
real    0m1.253s

$ agent-browser snapshot
- main
  - heading "droid-observatory readiness" [level=1, ref=e1]
  - button "clicks: 0" [ref=e2]
    - StaticText "clicks: 0"

$ agent-browser click @e2 && agent-browser snapshot -i
- heading "droid-observatory readiness" [level=1, ref=e1]
- button "clicks: 1" [ref=e2]

$ agent-browser screenshot /tmp/obs-readiness/shot-3141.png
  Screenshot saved to /tmp/obs-readiness/shot-3141.png     (12217 bytes)

$ agent-browser console      -> (empty)
$ agent-browser errors       -> (empty)
$ agent-browser get title    -> readiness
```

A second isolated session (`agent-browser --session val2 open http://localhost:3141`) launched its own
Chrome in `1.438s` and got an independent page state (`clicks: 0` while the default session showed
`clicks: 1`). `agent-browser session list` showed `default` and `val2`.

Resource samples while trunk serve and two browser sessions were running (ps, KB):

```text
# during the cold wasm build (t+19.7 s): 6 rustc processes, 43-72 %CPU each, RSS 120-325 MB each
85405 ... 726896  98,7 rustc --crate-name web_sys     (t+40 s; largest single rustc seen: 727 MB)
85562 ... 690256  99,1 rustc --crate-name tachys      (t+50 s)
# steady state after build:
83813  trunk serve                       RSS 34944 KB   0,1 %CPU
daemon 85721 chrome 85722 procs 11 total_rss_kb 1340832 sum_pct_cpu 0    (session default)
daemon 85871 chrome 85872 procs 12 total_rss_kb 1542128 sum_pct_cpu 1    (session val2)
# Chrome launch burst (first ~2 s): browser 40,2 %CPU, GPU helper 24,7 %CPU, renderers 4-17 %CPU
```

Summed RSS over a Chrome process tree double-counts shared pages; treat 1.3-1.5 GB per session as an
upper bound. Idle CPU of a session is 0-1 %.

Wasm artifacts in the shared target dir after the run:

```text
657M  /Users/dct/.cargo/shared_rust_target/wasm32-unknown-unknown
380M  /Users/dct/.cargo/shared_rust_target/wasm32-unknown-unknown/debug
277M  /Users/dct/.cargo/shared_rust_target/wasm32-unknown-unknown/release
564K  /Users/dct/.cargo/shared_rust_target/wasm-bindgen
364K  /tmp/obs-readiness/leptos-tauri/ui/dist   (ui-*_bg.wasm 331207 bytes, ui-*.js 28785 bytes, index.html 5028 bytes)
6,8G  /Users/dct/.cargo/shared_rust_target  (debug 1,9G, release 4,3G; was 6,1G before this run)
```

Shutdown: `agent-browser close --all` closed both sessions; `kill 83813` stopped trunk;
`lsof -nP -iTCP:3141 -sTCP:LISTEN` then exited 1 (port free), no `trunk`/`agent-browser`/
`--remote-debugging-port` processes left.

## Tauri path

Command (background, log to file):

```sh
cd /tmp/obs-readiness/leptos-tauri
cargo tauri dev --config src-tauri/tauri.conf.json > /tmp/obs-readiness/tauri-dev.log 2>&1
```

`beforeDevCommand` spawns `trunk serve --config ui/Trunk.toml` (port 3141 from Trunk.toml), waits
for it, then runs `cargo run --no-default-features`. Timeline from launch (wasm was warm from step 1):

- trunk serve listening at 3141: log timestamp `14:57:01.99`, about 2 s after launch.
- native build: `Finished dev profile [optimized + debuginfo] target(s) in 31.18s`, then
  `Running /Users/dct/.cargo/shared_rust_target/debug/src-tauri`.
- app binary process present: **37.66 s** after launch (PID 86243, RSS 130800 KB, 0.0 %CPU idle).
  The debug binary is named `src-tauri` (the crate name), not `obs-readiness`.

Evidence of the window that worked non-interactively:

```text
$ osascript -e 'tell application "System Events" to get name of every process whose background only is false'
  ... src-tauri ...                      (exit 0, the app is listed as a foreground app)
$ osascript -e 'tell application "System Events" to get {name, unix id, frontmost, visible} of process "src-tauri"'
  src-tauri, 86243, false, true
$ lsof -nP -iTCP:3141
  com.apple 86479 ... TCP [::1]:55422->[::1]:3141 (ESTABLISHED)   (x3: the WKWebView networking process fetched the page)
$ time screencapture -x /tmp/obs-readiness/tauri-window.png
  real 0m0.228s, exit=0, 201367 bytes, 4112x2658 px, no permission prompt
```

Evidence that did NOT work in this run:

```text
$ osascript -e 'tell application "System Events" to tell process "src-tauri" to count windows'   -> 0
$ ffmpeg signalstats on tauri-window.png  -> YAVG=16 YMAX=16  (uniformly black image)
$ ioreg -n Root -d1 -a | grep -A1 CGSSession
  <key>CGSSessionScreenIsLocked</key> <true/>   (locked since epoch 1792680092)
$ osascript -l JavaScript -e 'ObjC.import("ApplicationServices"); $.AXIsProcessTrusted()'  -> true
```

Reading: Accessibility is granted to the terminal chain (iTerm2 -> zsh -> droid), `screencapture`
ran without a prompt, but the screen was locked for the whole run, so the capture is black and the
window list is empty. Whether a window-level screenshot works on an unlocked screen is therefore
unverified. Rerun `screencapture -x` and `count windows` once with the screen unlocked before the
mission relies on them. The window title to expect is `obs-readiness` (from tauri.conf.json), and
on the real app it will be whatever `app.windows[].title` says.

agent-browser cannot attach to the Tauri window: the app opens no CDP/TCP listener
(`lsof -nP -a -p 86243 -iTCP` empty), and `agent-browser --session tauri connect 9229` fails with
`All CDP discovery methods failed ... Connection refused`. WKWebView has no Chrome DevTools Protocol
endpoint; agent-browser drives Chromium/Electron only.

Practical Tauri validation evidence, in order of strength:

1. Same UI validated in Chromium through trunk serve on 3141 (functional evidence; the Tauri shell
   loads the identical `devUrl`).
2. `cargo tauri dev` log contains `Running .../debug/<crate>` with no panic afterwards, and the
   process stays alive (`ps -p <pid>`; measured RSS 130 MB).
3. `lsof -nP -iTCP:3141` shows `com.apple.WebKit` ESTABLISHED connections to trunk (the webview
   fetched the page).
4. System Events lists the process with `background only is false` and `visible true`; with the
   screen unlocked, `count windows` should be 1 and `name of windows` the configured title.
5. `screencapture -x` of the screen (unlocked only); window-only capture needs the CGWindow id,
   which this machine's `python3` cannot obtain (`No module named 'Quartz'`); use Swift or JXA.
6. For true in-window interaction, the mission would need a tauri-driver/WebDriver setup, which is
   out of scope here and not supported by tauri-driver on macOS at the moment of writing.

Shutdown gotcha: `kill <cargo-tauri pid>` left `trunk serve` and the app binary running,
reparented to PID 1:

```text
--- after kill 86056 (cargo-tauri)
86173     1 trunk serve --config ui/Trunk.toml
86243     1 /Users/dct/.cargo/shared_rust_target/debug/src-tauri
```

Both had to be killed by PID. Kill the whole process group (`kill -- -<pgid>`) or kill the three
PIDs. After that: no `cargo-tauri`/`trunk`/`src-tauri` processes, `lsof 3141` exit 1, System Events
no longer lists `src-tauri`.

## Concurrency and cost

Measured surfaces (RSS and CPU from ps, build times from cargo output):

| Surface | Startup | Steady state | Build burst | Cost |
|---|---|---|---|---|
| `trunk serve` (dev wasm, cold) | 55.9 s to first HTTP 200 | 35 MB RSS, 0.1 %CPU | up to 7 rustc at 40-99 %CPU each, largest rustc 727 MB RSS | medium during build, low idle |
| `trunk serve` incremental rebuild | 1.5-2.1 s | same | 1-2 rustc | low |
| agent-browser session (Chrome, headless) | 1.25-1.44 s | 1.34-1.54 GB summed RSS (11-12 procs), 0-1 %CPU idle | 40 %CPU browser + 25 %CPU GPU helper for ~2 s at launch | medium (memory), low (CPU) |
| `cargo tauri dev` (native, warm wasm) | 37.7 s to app process (31.2 s cargo build) | app 130 MB RSS, 0.0 %CPU; trunk child 35 MB | native rustc burst, serialized after trunk's build | medium during build, low idle |
| `cargo test` (core-deps scratch, warm) | 2.6 s | - | - | low when warm |
| `cargo llvm-cov --summary-only` (core-deps, separate `llvm-cov-target` dir) | 44.3 s | - | peak 7 rustc, 1.27 GB summed rustc RSS, sum 396 %CPU; load 1-min went 3.3 -> 14.6 | high |

Lock behaviour observed:

- trunk rebuild concurrent with a native `cargo build` in another crate: both logged
  `Blocking waiting for file lock on package cache`, trunk additionally logged
  `Blocking waiting for file lock on build directory`. They serialize; neither failed.
- `cargo llvm-cov` builds under `$CARGO_TARGET_DIR/llvm-cov-target`, so it did not block on the
  build-directory lock held by trunk (0 `Blocking` lines in its log) and only shares the brief
  package-cache lock. It does compete for CPU: the load average hit 14.6 on 10 cores.
- Running `cargo tauri dev` already serializes internally: trunk's wasm build takes the lock first,
  then `cargo run` for the native crate.

70 % headroom rule (use at most 7 cores and 44.8 GB; baseline load 3-6 and about 60 % memory
free per `memory_pressure`, i.e. about 38 GB usable):

- (a) Concurrent agent-browser validator subagents against one trunk serve: **3**. Each session is
  about 1.5 GB RSS upper bound and near-zero idle CPU, so memory allows more, but each validator
  also runs an LLM-driven CLI loop and the page is a single shared dev server with a live-reload
  websocket; three keeps total browser memory under 5 GB and leaves CPU for one build. Use
  `--session <name>` per validator; sessions are isolated (verified above). Do not exceed 5.
- (b) `trunk serve` and `cargo tauri dev` coexisting: **yes, but run one trunk only**. `cargo tauri
  dev` starts its own `trunk serve` on 3141 via `beforeDevCommand`; a separately started trunk on
  3141 would make the second one fail to bind. Either run only `cargo tauri dev` (its trunk serves
  the browser validators too, verified: `curl` on 3141 answered while the Tauri window was up) or
  point the standalone trunk at a scratch port 3142-3149. Compiles serialize on the shared target
  lock; expect the native build (31 s warm deps) to wait for any in-flight wasm build.
- (c) `cargo test`, `cargo llvm-cov`, `trunk serve` at the same time: **technically possible,
  recommend serializing llvm-cov**. `cargo test` and trunk share the build-directory lock and
  serialize automatically (safe, just slower). `cargo llvm-cov` uses its own target dir, so it runs
  truly in parallel and pushed the 1-min load from 3.3 to 14.6 on 10 cores, above the 7-core
  budget. Run llvm-cov alone (or only with idle servers), and let `cargo test` and trunk rebuilds
  serialize on the lock.

Memory check: at the peak of the concurrent test (7 rustc, 1.27 GB) plus two Chrome sessions
(about 2.9 GB) plus the Tauri app (130 MB), `memory_pressure` still reported 60 % free. Memory is
not the constraint on this machine; CPU and the shared target lock are.

## Gotchas

- `CARGO_TARGET_DIR=/Users/dct/.cargo/shared_rust_target` comes from the shell environment, not
  from `~/.cargo/config.toml`. All cargo-driven builds (trunk, tauri, test) share one build
  directory lock and log `Blocking waiting for file lock on build directory` when they collide.
  `cargo llvm-cov` is the exception (own `llvm-cov-target` subdir).
- `~/.cargo/config.toml` sets `[profile.dev] opt-level = 1, debug = 1` globally; the dev wasm
  build is therefore `[optimized + debuginfo]`, not a plain debug build. The mission's own profile
  settings in Cargo.toml will be merged with this.
- sccache is the rustc wrapper; its Rust hit rate during this session was 0.30 % (`Cache hits
  (Rust) 6`, `Cache misses (Rust) 1993`), so it gives no speedup for fresh dependency sets yet.
- Trunk flags: `--open false` (not `--open=false`), `--config ui/Trunk.toml` when the Trunk.toml is
  not in the cwd. Trunk 0.22.0 prints a `starting build` line and `success` when ready; the
  `server listening at` block also lists every `/etc/hosts`-style name (CRC OpenShift entries
  here), which is noise.
- Dev page HTML contains trunk's live-reload script (websocket to `/.well-known/trunk/ws`); a
  validator that reads the raw HTML must look at `<main>`, not the top of `<body>`. `snapshot`
  already does this.
- agent-browser: no first-run download (`agent-browser doctor` found Google Chrome 155 at
  `/Applications` and reported `Headless launch + about:blank in 1.45s`). State lives in
  `~/.agent-browser/` (36 KB). A daemon lingers per session (default idle timeout 1 h); use
  `agent-browser close --all` at the end of a validation. A failed `connect` still leaves a
  `tauri` daemon entry; close it too. `agent-browser doctor` prints `No encryption key set`
  (informational, only relevant for `auth save`).
- `cargo tauri dev` process tree: killing `cargo-tauri` alone orphans `trunk serve` and the app
  binary (both reparent to PID 1 and keep running, trunk keeps 3141 bound). Kill all three PIDs or
  the process group.
- The debug app binary is `$CARGO_TARGET_DIR/debug/<crate-name>` (`src-tauri` here); match the
  process by that path, and note the `productName` only affects the bundle, not the dev binary.
- Screen lock: this run happened with the screen locked (`CGSSessionScreenIsLocked true`), so
  `screencapture -x` produced a black 4112x2658 image (luma 16 everywhere) and System Events
  reported 0 windows for the app. No permission prompt appeared; Accessibility is already trusted
  for the iTerm2 chain (`AXIsProcessTrusted() true`). Validators that need a Tauri screenshot
  require an unlocked, logged-in display; schedule those steps accordingly or accept
  process/log/System-Events evidence instead.
- `python3` on PATH has no `Quartz` (pyobjc) module, so CGWindowList enumeration must use JXA or
  Swift if a window-only capture (`screencapture -l <windowid>`) is wanted.
- `top` is rejected by the agent runtime as interactive; sample with
  `ps -axo pid,ppid,rss,%cpu,command` and `sysctl -n vm.loadavg`. `ps` prints decimal commas
  (`43,0`) in this locale; convert before summing.
- Timing numbers: cold dev wasm build 53.64 s (190 crates); warm native Tauri build 31.18 s;
  incremental wasm rebuild 1.46-2.08 s; agent-browser open 1.25-1.44 s; llvm-cov on a 14-dep
  scratch crate 44.3 s; trunk ready-to-curl 55.94 s; tauri launch-to-app-process 37.66 s.

## Exact commands that worked

```sh
# Browser path
cd /tmp/obs-readiness/leptos-tauri
trunk serve --config ui/Trunk.toml --port 3141 --open false > /tmp/obs-readiness/trunk-serve.log 2>&1 &
until curl -sf -o /dev/null http://localhost:3141/; do sleep 1; done
agent-browser open http://localhost:3141
agent-browser snapshot
agent-browser snapshot -i
agent-browser click @e2
agent-browser get title
agent-browser get text body
agent-browser screenshot /tmp/obs-readiness/shot-3141.png
agent-browser console
agent-browser errors
agent-browser --session val2 open http://localhost:3141      # isolated second validator
agent-browser session list
agent-browser close --all
kill <trunk pid>
lsof -nP -iTCP:3141 -sTCP:LISTEN   # exit 1 = free

# Tauri path
cd /tmp/obs-readiness/leptos-tauri
cargo tauri dev --config src-tauri/tauri.conf.json > /tmp/obs-readiness/tauri-dev.log 2>&1 &
# wait for the app binary:
ps -axo pid,command | awk '$2 ~ /shared_rust_target\/debug\/src-tauri$/ {print $1}'
osascript -e 'tell application "System Events" to get name of every process whose background only is false'
osascript -e 'tell application "System Events" to get {name, unix id, frontmost, visible} of process "src-tauri"'
osascript -e 'tell application "System Events" to tell process "src-tauri" to count windows'   # expect 1 when unlocked
lsof -nP -iTCP:3141                 # com.apple.WebKit ESTABLISHED lines = webview loaded the page
screencapture -x /tmp/obs-readiness/tauri-window.png   # unlocked screen only
ffmpeg -hide_banner -i /tmp/obs-readiness/tauri-window.png -vf signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=- -f null - 2>/dev/null | grep YAVG
ioreg -n Root -d1 -a | grep -A1 CGSSessionScreenIsLocked
# shutdown: kill cargo-tauri AND its two children
kill <cargo-tauri pid> <trunk pid> <app pid>

# Resource sampling
ps -axo pid,ppid,rss,%cpu,command | grep -E 'trunk|rustc --crate-name|cargo|agent-browser|remote-debugging-port'
sysctl -n vm.loadavg
memory_pressure | tail -3
du -sh ~/.cargo/shared_rust_target/wasm32-unknown-unknown
sccache --show-stats
```

Artifacts: `/tmp/obs-readiness/shot-3141.png`, `/tmp/obs-readiness/tauri-window.png` (black, locked
screen), `/tmp/obs-readiness/trunk-serve.log`, `/tmp/obs-readiness/tauri-dev.log`,
`/tmp/obs-readiness/concurrent-cargo-build.log`, `/tmp/obs-readiness/concurrent-cargo-test.log`,
`/tmp/obs-readiness/concurrent-llvm-cov.log`.

## Owner decisions (2026-10-22)

- Tauri evidence: the validators' smoke check is accepted as evidence for the Tauri path (process
  alive, `Running .../debug/<crate>` log line with no panic, `com.apple.WebKit` ESTABLISHED
  connections on 3141, System Events listing the process). At each milestone integration the owner
  takes one real screenshot with the display unlocked and attaches it to the report, so the evidence
  set has one human-captured frame.
- Concurrency limits: accepted as measured above and written into `SPEC.md` §5.8 (runtime limits)
  and `AGENTS.md` ("Runtime limits (measured)") so workers do not rediscover them: at most 3
  concurrent agent-browser validator sessions per `trunk serve`; `cargo llvm-cov` runs alone; all
  cargo builds serialize on the shared `CARGO_TARGET_DIR` lock; exactly one trunk owns port 3141.

Related: `2026-10-22-dependency-readiness.md` (toolchain, crates, OTLP, GitHub, session schema).
