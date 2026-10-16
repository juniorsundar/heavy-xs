#!/usr/bin/env sh
# Build, stage, run the headless contract runtime, and diff against the recorded C baseline.
#   scripts/rust-port/verify.sh [suite,suite,...]   verify (default: smoke)
#   RECORD=1 scripts/rust-port/verify.sh suite      (re)record the baseline for each suite
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
build=${BUILD_DIR:-$root/build}
stage=$build/stage/usr/local
# Default: every suite that has a recorded baseline.
suites=${1:-$(ls "$root/scripts/rust-port/baseline" 2>/dev/null | sed "s/\.txt$//" | tr "\n" "," | sed "s/,$//")}

if ! ninja -C "$build"; then
  exit 1
fi
meson install -C "$build" --destdir "$build/stage" >/dev/null 2>&1

# Install the harness next to the data dir so `require` finds it without
# touching anything in data/. The editor selects it with LITE_XL_RUNTIME.
share=$stage/share/lite-xl
mkdir -p "$share/contract_cases"
cp "$root/tests/contract/contract.lua" "$share/contract.lua"
cp "$root"/tests/contract/cases/*.lua "$share/contract_cases/"

# Suites that inject synthetic SDL events need the Rust `test_hooks` feature
# (rust/src/test_hooks.rs). It is enabled for the run and restored afterwards, so a
# normal build never ships with it.
needs_hooks=0
for s in $(echo "$suites" | tr , ' '); do
  case $s in sysevents|syswindow|sysplugin|customevents|rencache) needs_hooks=1 ;; esac
done
prev_features=$(meson introspect "$build" --buildoptions 2>/dev/null | tr ',' '\n' | grep -A1 '"name": "core_features"' | grep -o '"value": "[^"]*"' | head -1 | cut -d'"' -f4 || true)
if [ "$needs_hooks" = 1 ]; then
  meson configure "$build" -Dcore_features=test_hooks >/dev/null
  ninja -C "$build" >/dev/null
  meson install -C "$build" --destdir "$build/stage" >/dev/null 2>&1
  trap 'meson configure "$build" -Dcore_features="${prev_features:-}" >/dev/null; ninja -C "$build" >/dev/null' EXIT
fi

status=0
for s in $(echo "$suites" | tr , ' '); do
  out=$(mktemp)
  # A crash or non-zero exit must fail the run, not silently truncate the diff.
  # RECORD=1 on an event suite records the C poll_event/wait_event (the same binary
  # can run either; see rust/src/system.rs).
  c_events=
  if [ "${RECORD:-}" = 1 ]; then case $s in sysevents|syswindow|sysplugin|customevents) c_events=1 ;; esac; fi
  # Never let a suite open a real dialog on the developer's desktop: no session bus (the
  # portal is reached over D-Bus) and no forced dialog driver, so SDL answers with an
  # "error" event instead. This applies to every suite, not just the dialog one.
  if DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/lxl-contract-bus SDL_FILE_DIALOG_DRIVER= \
      LXL_PLUGIN_SRC=$root/tests/contract/plugin LXL_PLUGIN_INC=$root/resources/include \
      LXL_TEST_C_EVENTS=${c_events:-} CONTRACT_SUITES=$s SDL_VIDEODRIVER=${SDL_VIDEODRIVER:-offscreen} \
      LITE_PREFIX=$stage LITE_XL_RUNTIME=contract \
      timeout 60 "$build/src/lite-xl" >"$out" 2>&1; then
    rc=0
  else
    rc=$?
    echo "exit status $rc" >>"$out"
  fi
  base=$root/scripts/rust-port/baseline/$s.txt
  if [ "${RECORD:-}" = 1 ]; then
    # A suite that errored or crashed must never become a baseline.
    if [ "$rc" != 0 ]; then
      echo "REFUSING to record $s: exit $rc"; sed -n '$p' "$out"; status=1
    else
      cp "$out" "$base"; echo "recorded $base"
    fi
  elif [ "$rc" != 0 ]; then
    echo "FAIL $s (exit $rc)"; sed -n '$p' "$out"; status=1
  elif diff -u "$base" "$out"; then
    echo "OK   $s"
  else
    echo "FAIL $s"; status=1
  fi
  rm -f "$out"
done
exit $status
