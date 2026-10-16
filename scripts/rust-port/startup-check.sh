#!/usr/bin/env sh
# Checks what rust/src/app.rs owns and the module suites cannot reach: the globals handed to
# start.lua, the restart loop (Lua state recreated, window kept), exit codes and error.txt.
# Runs the staged binary left behind by verify.sh against throwaway runtimes.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
target=${TARGET_DIR:-$root/rust/target}
case " ${CARGO_ARGS:-} " in *" --release "*) profile=release ;; *) profile=debug ;; esac
bin=$target/$profile/lite-xl
stage=${STAGE_DIR:-$target/stage}/usr/local
share=$stage/share/lite-xl
[ -x "$bin" ] && [ -d "$share" ] || { echo "run scripts/rust-port/verify.sh first"; exit 1; }

work=$(mktemp -d)
# The runtimes below live in the staged install; remove them even on interrupt.
trap 'rm -rf "$work" "$share/startup_restart.lua" "$share/startup_broken.lua"' EXIT INT TERM
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
( cd "$work" && DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent SDL_VIDEODRIVER=dummy \
  LITE_PREFIX=$stage LITE_XL_RUNTIME=startup_restart timeout -k 2 "${STARTUP_TIMEOUT:-30}" "$bin" extra-arg >"$out" 2>&1 ) && rc=0 || rc=$?
check "restart: exit code" "$rc" 0
check "restart: two runs" "$(grep -c '^STARTUP run' "$out")" 2
check "restart: first run not RESTARTED" "$(grep -c '^STARTUP run false args 2 extra-arg platform Linux arch ok exe lite-xl datadir true' "$out")" 1
check "restart: second run RESTARTED" "$(grep -c '^STARTUP run true ' "$out")" 1
check "restart: window survives" "$(grep -c '^STARTUP restored yes 200' "$out")" 1

# --- startup failure: a runtime that cannot load must exit 1 and leave error.txt. The fatal
# error dialog is attempted, so the process gets no display or bus that could show a real one.
cat > "$share/startup_broken.lua" <<'LUA'
error("deliberate startup failure")
LUA
out=$work/broken.log
( cd "$work" && env -u DISPLAY -u WAYLAND_DISPLAY -u XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent \
  SDL_VIDEODRIVER=offscreen LITE_PREFIX=$stage LITE_XL_RUNTIME=startup_broken timeout -k 2 "${STARTUP_TIMEOUT:-30}" "$bin" >"$out" 2>&1 ) && rc=0 || rc=$?
check "failure: exit code" "$rc" 1
check "failure: error.txt names the error" "$(grep -c 'deliberate startup failure' "$work/error.txt" 2>/dev/null || echo 0)" 1
check "failure: stdout has traceback" "$(grep -c 'stack traceback' "$out")" 1

exit $fail
