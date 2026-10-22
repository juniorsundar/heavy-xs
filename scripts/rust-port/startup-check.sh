#!/usr/bin/env sh
# Checks what src/app.rs owns and the module suites cannot reach: the globals handed to
# start.lua, the restart loop (Lua state recreated, window kept), exit codes and error.txt.
# Runs the staged binary left behind by verify.sh (or BIN with its STAGE_DIR) against throwaway
# runtimes, each a fresh process in a runner-owned environment (private-display.sh).
#   STARTUP_PROFILE  dummy (default: SDL's dummy driver) | xvfb (private Xvfb) for restart and quit;
#                    the startup-failure case always runs with no display at all
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/scripts/rust-port/private-display.sh"
target=${TARGET_DIR:-$root/target/contract}
case " ${CARGO_ARGS:-} " in *" --release "*) profile=release ;; *) profile=debug ;; esac
bin=${BIN:-$target/$profile/lite-xl}
stage=${STAGE_DIR:-$target/stage}/usr/local
share=$stage/share/lite-xl
[ -x "$bin" ] && [ -d "$share" ] || { echo "run scripts/rust-port/verify.sh first"; exit 1; }

work=$(mktemp -d)
# The runtimes below live in the staged install; remove them even on interrupt.
cleanup() { pd_profile_stop; rm -rf "$work" "$share/startup_restart.lua" "$share/startup_broken.lua" "$share/startup_sandbox.lua"; }
trap cleanup EXIT
trap 'exit 130' INT TERM
pd_profile_start "${STARTUP_PROFILE:-dummy}" "$work/display"
pd_describe
fail=0
check() { if [ "$2" = "$3" ]; then echo "OK   $1"; else echo "FAIL $1: expected [$3] got [$2]"; fail=1; fi; }

# --- globals + restart: run 1 persists a window and asks for a restart, run 2 must see
# RESTARTED=true and get the same window back from renwindow._restore().
cat > "$share/startup_restart.lua" <<'LUA'
local M = {}
local function say(...) io.stderr:write("STARTUP ", table.concat({...}, " "), "\n") end
function M.init()
  say("run", tostring(RESTARTED), "args", #ARGS, ARGS[2] or "-", "platform", PLATFORM,
      "arch", type(ARCH) == "string" and "ok" or "bad", "exe", EXEFILE:match("[^/]+$"),
      "datadir", tostring(DATADIR == os.getenv("LITE_PREFIX") .. "/share/lite-xl"))
  if not RESTARTED then
    local w = renwindow.create("startup-check", 200, 100)
    w:_persist()
    M.restart_request = true
  else
    local w = renwindow._restore()
    say("restored", w and "yes" or "no", w and select(1, w:get_size()) or 0)
    M.restart_request = false
  end
end
function M.run() end
function M.on_error(e) say("error", tostring(e)) end
return M
LUA
# SDL's "dummy" video driver, not "offscreen": the persisted window is never destroyed (by design),
# and exiting with a live window under "offscreen" segfaults in SDL_Quit. Upstream's C build does
# the same (checked at 234fa099, 10/10), so it is not part of this port.
out=$work/restart.log
( cd "$work" && pd_run LITE_PREFIX="$stage" LITE_XL_RUNTIME=startup_restart timeout -k 2 "${STARTUP_TIMEOUT:-30}" "$bin" extra-arg >"$out" 2>&1 ) && rc=0 || rc=$?
check "restart: exit code" "$rc" 0
check "restart: two runs" "$(grep -c '^STARTUP run' "$out")" 2
check "restart: first run not RESTARTED" "$(grep -c '^STARTUP run false args 2 extra-arg platform Linux arch ok exe lite-xl datadir true' "$out")" 1
check "restart: second run RESTARTED" "$(grep -c '^STARTUP run true ' "$out")" 1
check "restart: window survives" "$(grep -c '^STARTUP restored yes 200' "$out")" 1

# --- sandbox classification: get_sandbox reads the environment as it was at process start (SDL's
# snapshot), updated only by system.setenv, and caches its first answer across a Lua restart.
cat > "$share/startup_sandbox.lua" <<'LUA'
local M = {}
local function say(...) io.stderr:write("SANDBOX ", table.concat({...}, " "), "\n") end
function M.init()
  if not RESTARTED then
    say("first", system.get_sandbox())
    system.setenv("SNAP_REVISION", "1") -- completes the set, but the answer is already cached
    M.restart_request = true
  else
    say("again", system.get_sandbox())
    M.restart_request = false
  end
end
function M.run() end
function M.on_error(e) say("error", tostring(e)) end
return M
LUA
sandbox_run() { # name, env assignments...
  local name=$1; shift
  ( cd "$work" && pd_profile=dummy && pd_run LITE_PREFIX="$stage" LITE_XL_RUNTIME=startup_sandbox "$@" timeout -k 2 "${STARTUP_TIMEOUT:-30}" "$bin" >"$work/sandbox_$name.log" 2>&1 ) || true
}
sandbox_run none
check "sandbox: none, cached across restart" "$(grep '^SANDBOX' "$work/sandbox_none.log" | tr '\n' ' ')" "SANDBOX first none SANDBOX again none "
sandbox_run snap SNAP= SNAP_NAME=n SNAP_REVISION=1
check "sandbox: snap needs all three (empty counts)" "$(grep '^SANDBOX first' "$work/sandbox_snap.log")" "SANDBOX first snap"
sandbox_run partial SNAP=1 SNAP_NAME=n
check "sandbox: setenv after start is too late once cached" "$(grep '^SANDBOX' "$work/sandbox_partial.log" | tr '\n' ' ')" "SANDBOX first none SANDBOX again none "

# --- startup failure: a runtime that cannot load must exit 1 and leave error.txt. The fatal
# error dialog is attempted, so the process gets no display or bus that could show a real one.
cat > "$share/startup_broken.lua" <<'LUA'
error("deliberate startup failure")
LUA
out=$work/broken.log
# pd_run without the xvfb display: the same isolation, offscreen driver, nothing to show a dialog on.
( cd "$work" && pd_profile=offscreen && pd_run LITE_PREFIX="$stage" LITE_XL_RUNTIME=startup_broken timeout -k 2 "${STARTUP_TIMEOUT:-30}" "$bin" >"$out" 2>&1 ) && rc=0 || rc=$?
check "failure: exit code" "$rc" 1
check "failure: error.txt names the error" "$(grep -c 'deliberate startup failure' "$work/error.txt" 2>/dev/null || echo 0)" 1
check "failure: stdout has traceback" "$(grep -c 'stack traceback' "$out")" 1

# --- quit: Ctrl-C (SIGINT, which SDL turns into a quit event) removes every project in core.exit, and
# core.run stops after that step. The step must not update or draw in between, or any plugin that
# calls core.root_project() during update/draw raises (seen with lintplus and lsp).
mkdir -p "$work/quituser" "$work/quitproj"
echo 'local x = 1' > "$work/quitproj/a.lua"
cat > "$work/quituser/init.lua" <<'LUA'
local core = require "core"
local RootView = require "core.rootview"
for _, name in ipairs { "update", "draw" } do
  local orig = RootView[name]
  RootView[name] = function(...)
    if not core.root_project() then io.stderr:write("QUIT ", name, " without a project\n") end
    return orig(...)
  end
end
core.add_thread(function()
  coroutine.yield(0.5)
  io.stderr:write("QUIT ready\n")
end)
LUA
out=$work/quit.log
# A non-interactive shell starts background jobs with SIGINT ignored, and the editor would inherit
# that (it then only ends at the timeout's SIGTERM); --default-signal gives it a terminal's Ctrl-C.
( cd "$work/quitproj" && pd_exec=1 && pd_run env --default-signal=INT \
  LITE_USERDIR="$work/quituser" LITE_PREFIX="$stage" "$bin" "$work/quitproj/a.lua" >"$out" 2>&1 ) &
pid=$!
i=0; while [ $i -lt 100 ] && ! grep -q '^QUIT ready' "$out" 2>/dev/null; do sleep 0.1; i=$((i + 1)); done
kill -INT "$pid"
# Must end on its own well before any timeout would.
i=0; while [ $i -lt 50 ] && kill -0 "$pid" 2>/dev/null; do sleep 0.1; i=$((i + 1)); done
if kill -0 "$pid" 2>/dev/null; then kill -KILL "$pid"; fi
wait "$pid" && rc=0 || rc=$?
check "quit: SIGINT reached a running editor" "$(grep -c '^QUIT ready' "$out")" 1
check "quit: exits 0 within 5s of SIGINT" "$rc" 0
check "quit: no update/draw after projects removed" "$(grep -c '^QUIT .* without a project' "$out")" 0
check "quit: exits cleanly" "$([ -e "$work/quituser/error.txt" ] && echo error.txt || echo clean)" clean

exit $fail
