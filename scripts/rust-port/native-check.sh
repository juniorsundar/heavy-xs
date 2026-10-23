#!/usr/bin/env sh
# Native OS-path checks on a runner-owned Xvfb and display-free child processes.
# Uses the staged binary from verify.sh (BIN/STAGE_DIR/TARGET_DIR as in startup-check.sh).
# XTEST drives X11 through its real native event path; it is not physical-hardware evidence.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/scripts/rust-port/private-display.sh"
target=${TARGET_DIR:-$root/target/contract}
case " ${CARGO_ARGS:-} " in *" --release "*) build=release ;; *) build=debug ;; esac
bin=${BIN:-$target/$build/lite-xl}
stage=${STAGE_DIR:-$target/stage}/usr/local
share=$stage/share/lite-xl
[ -x "$bin" ] && [ -d "$share" ] || { echo 'run verify.sh first' >&2; exit 1; }
work=$(mktemp -d)
subject_pid=
cleanup() {
  if [ -n "$subject_pid" ]; then kill -KILL "$subject_pid" 2>/dev/null || true; wait "$subject_pid" 2>/dev/null || true; fi
  pd_profile_stop
  rm -rf "$work" "$share/native_check.lua"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
cp "$root/tests/contract/native.lua" "$share/native_check.lua"
pd_profile_start xvfb "$work/display"
pd_describe
# Test-only XTest driver; the dev shell provides its libraries, and nothing here is shipped.
# shellcheck disable=SC2046
cc -Wall -Wextra -Werror "$root/tests/contract/native-input.c" -o "$work/input-driver" $(pkg-config --cflags --libs x11 xtst)
fail=0
check() { if [ "$2" = "$3" ]; then echo "OK   $1"; else echo "FAIL $1: expected [$3] got [$2]"; fail=1; fi; }
ready() {
  i=0
  while [ $i -lt 100 ] && [ ! -f "$1" ]; do
    kill -0 "$subject_pid" 2>/dev/null || break
    sleep 0.1; i=$((i + 1))
  done
  [ -f "$1" ]
}
start() {
  mode=$1
  run=$work/$mode
  mkdir -p "$run/user"
  rm -f "$run/ready" "$run/focused"
  ( cd "$run"
    case $mode in preloop|failed) pd_profile=offscreen ;; esac
    pd_exec=1
    pd_run env --default-signal=INT --default-signal=TERM \
      NATIVE_CHECK_MODE="$mode" NATIVE_CHECK_WORK="$run" LITE_USERDIR="$run/user" \
      LITE_PREFIX="$stage" LITE_XL_RUNTIME=native_check "$bin" >"$run/log" 2>&1
  ) &
  subject_pid=$!
}
finish() {
  i=0
  while [ $i -lt 50 ] && kill -0 "$subject_pid" 2>/dev/null; do sleep 0.1; i=$((i + 1)); done
  if kill -0 "$subject_pid" 2>/dev/null; then kill -KILL "$subject_pid"; fi
  wait "$subject_pid" && rc=0 || rc=$?
  subject_pid=
  check "$mode: exit code" "$rc" 0
  check "$mode: completion" "$(grep -c '^NATIVE .*passed$' "$run/log")" 1
  if [ "$rc" != 0 ]; then printf '%s\n' "--- $mode log"; sed 's/^/    /' "$run/log"; fi
}
for mode in preloop failed window; do
  for sig in INT TERM; do
    start "$mode"
    if ready "$run/ready"; then
      kill -"$sig" "$subject_pid"
    else
      echo "FAIL $mode SIG$sig: subject not ready"; fail=1
    fi
    finish
  done
done
start input
if ready "$run/ready" && pd_run "$work/input-driver" focus && ready "$run/focused"; then
  pd_run "$work/input-driver" keys
else
  echo 'FAIL native input: focus handshake'; fail=1
fi
finish
exit $fail
