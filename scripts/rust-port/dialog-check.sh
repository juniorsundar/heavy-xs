#!/usr/bin/env sh
# Real dialog and fatal-notification transport on private, non-activating buses.
# No host desktop, portal, notification daemon or hook-enabled binary is required.
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
subject_pid= portal_pid= bus_pid= empty_bus_pid=
cleanup() {
  for pid in "$subject_pid" "$portal_pid" "$bus_pid" "$empty_bus_pid"; do
    if [ -n "$pid" ]; then kill -KILL "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi
  done
  pd_profile_stop
  rm -rf "$work" "$share/dialog_check.lua" "$share/fatal_check.lua"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
cp "$root/tests/contract/dialog.lua" "$share/dialog_check.lua"
cp "$root/tests/contract/fatal.lua" "$share/fatal_check.lua"
pd_profile_start xvfb "$work/display"
pd_describe
# Reuse the current Cargo target; read the exact fixture artifact rather than guessing a hash.
if ! env -u LD_LIBRARY_PATH CARGO_TARGET_DIR="$target" cargo test --manifest-path "$root/Cargo.toml" --offline --locked --lib --no-run --message-format=json >"$work/build.jsonl"; then
  jq -r 'select(.reason == "compiler-message") | .message.rendered' "$work/build.jsonl" >&2
  exit 1
fi
fixture=$(jq -r 'select(.reason == "compiler-artifact" and .target.name == "lite_xl_core" and .profile.test) | .executable // empty' "$work/build.jsonl")
[ -x "$fixture" ]
if ! pd_run env LXL_PRIVATE_DISPLAY_TEST=1 timeout -k 1 10 "$fixture" --exact native::tests::pump_unwind_keeps_parent_alive --ignored >"$work/unwind.log" 2>&1; then
  tail -n 30 "$work/unwind.log"; exit 1
fi
echo "OK   callback unwind: parent and display ownership restored"
# shellcheck disable=SC2046
cc -Wall -Wextra -Werror "$root/tests/contract/native-input.c" -o "$work/parent-check" $(pkg-config --cflags --libs x11 xtst)
# No service directories: the fake bus cannot activate a real portal/helper.
printf '%s\n' "<busconfig><type>session</type><listen>unix:path=$work/bus</listen><auth>EXTERNAL</auth><policy context=\"default\"><allow send_destination=\"*\"/><allow receive_sender=\"*\"/><allow own=\"*\"/></policy></busconfig>" >"$work/bus.conf"
dbus-daemon --nofork --config-file="$work/bus.conf" >"$work/bus.log" 2>&1 &
bus_pid=$!
i=0; while [ ! -S "$work/bus" ] && [ $i -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
[ -S "$work/bus" ]
bus=unix:path=$work/bus
(pd_exec=1; pd_run env DBUS_SESSION_BUS_ADDRESS="$bus" DIALOG_CHECK_WORK="$work" "$fixture" --exact portal_fixture::serve --ignored --nocapture >"$work/portal.log" 2>&1) &
portal_pid=$!
wait_file() {
  i=0
  while [ ! -f "$work/$1" ] && [ $i -lt 100 ]; do
    kill -0 "$portal_pid" 2>/dev/null || break
    if [ -n "$subject_pid" ]; then kill -0 "$subject_pid" 2>/dev/null || break; fi
    sleep 0.1; i=$((i + 1))
  done
  if [ ! -f "$work/$1" ]; then echo "FAIL barrier $1"; tail -n 25 "$work/portal.log"; return 1; fi
}
wait_file portal-ready
start() {
  role=$1
  mkdir -p "$work/$role/user"
  (cd "$work/$role"; pd_exec=1; pd_run env DBUS_SESSION_BUS_ADDRESS="$bus" DIALOG_CHECK_WORK="$work" DIALOG_CHECK_ROLE="$role" \
    LITE_USERDIR="$work/$role/user" LITE_PREFIX="$stage" LITE_XL_RUNTIME=dialog_check "$bin" >"$work/$role/log" 2>&1) &
  subject_pid=$!
}
finish() {
  i=0; while kill -0 "$subject_pid" 2>/dev/null && [ $i -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
  if kill -0 "$subject_pid" 2>/dev/null; then kill -KILL "$subject_pid"; fi
  wait "$subject_pid" && rc=0 || rc=$?
  subject_pid=
  if [ "$rc" != 0 ] || ! grep -q "^DIALOG $role passed$" "$work/$role/log"; then
    echo "FAIL $role exit=$rc"; tail -n 30 "$work/$role/log"; tail -n 30 "$work/portal.log"; exit 1
  fi
  echo "OK   $role: completion, exit and no blocked chooser teardown"
}
parent() {
  xid=$(cut -d: -f2 "$work/received-$1")
  pd_run "$work/parent-check" parent "$xid" "$2"
}
start normal
wait_file received-dup1; wait_file received-dup2
touch "$work/release-dup1" "$work/release-dup2"
wait_file received-held
parent held visible
touch "$work/parent-visible"
wait_file parent-closed
parent held hidden
echo 'OK   closed parent: hidden native handles retained'
touch "$work/release-held"
wait_file parent-released
parent held absent
echo 'OK   completed parent: last lease released on main pump'
touch "$work/parent-gone"
finish
start restart
wait_file received-restart; wait_file received-new
parent restart hidden
touch "$work/release-restart"
wait_file old-filtered
parent restart absent
echo 'OK   restart: retired result discarded, old parent released, caller id reusable'
touch "$work/release-new"
finish
start exit
wait_file received-exit
finish
kill -0 "$portal_pid"
echo 'OK   outstanding chooser: process exits without waiting for response'

# A second non-activating bus has no notification service at all.
printf '%s\n' "<busconfig><type>session</type><listen>unix:path=$work/empty-bus</listen><auth>EXTERNAL</auth><policy context=\"default\"><allow send_destination=\"*\"/><allow receive_sender=\"*\"/><allow own=\"*\"/></policy></busconfig>" >"$work/empty.conf"
dbus-daemon --nofork --config-file="$work/empty.conf" >"$work/empty.log" 2>&1 &
empty_bus_pid=$!
i=0; while [ ! -S "$work/empty-bus" ] && [ $i -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
[ -S "$work/empty-bus" ]
fatal() {
  mode=$1
  dir=$work/fatal-$mode
  mkdir -p "$dir/user"
  printf '%s' "$mode" >"$work/notify-mode"
  : >"$dir/stderr"
  error_output=$dir/stderr
  [ "$mode" != writefail ] || error_output=/dev/full
  address=$bus; test_profile=offscreen; disposition=--default-signal
  case $mode in
    absent) address=unix:path=$work/empty-bus ;;
    deadbus) address=unix:path=$work/no-bus ;;
    auth-timeout) address=unix:path=$work/auth-stall ;;
    independent) test_profile=xvfb ;;
    lazy-runtime) disposition=--ignore-signal ;;
  esac
  (cd "$dir"; pd_exec=1; pd_profile=$test_profile
    pd_run env "$disposition=INT" "$disposition=TERM" DBUS_SESSION_BUS_ADDRESS="$address" \
      DIALOG_CHECK_WORK="$work" FATAL_CHECK_ROLE="$mode" LITE_USERDIR="$dir/user" \
      LITE_PREFIX="$stage" LITE_XL_RUNTIME=fatal_check "$bin" >"$dir/stdout" 2>"$error_output") &
  subject_pid=$!
  i=0; while kill -0 "$subject_pid" 2>/dev/null && [ $i -lt 70 ]; do sleep 0.1; i=$((i + 1)); done
  if kill -0 "$subject_pid" 2>/dev/null; then kill -KILL "$subject_pid"; fi
  wait "$subject_pid" && rc=0 || rc=$?
  subject_pid=
  expected_rc=0
  [ "$mode" != bootstrap ] || expected_rc=1
  if [ "$rc" != "$expected_rc" ]; then
    echo "FAIL fatal $mode exit=$rc"; tail -n 20 "$dir/stdout" "$dir/stderr" "$work/portal.log"; exit 1
  fi
  if [ "$mode" = bootstrap ]; then
    grep -q 'stack traceback' "$dir/stdout"
    grep -q 'deliberate fatal notification startup failure' "$dir/error.txt"
  else
    grep -q "^FATAL $mode passed$" "$dir/stdout"
  fi
  caps=1; notify=1
  case $mode in
    invalid-title|invalid-message|absent|deadbus|auth-timeout) caps=0; notify=0 ;;
    caps-error|caps-timeout) notify=0 ;;
  esac
  for phase in caps caps-validated notify notify-validated; do
    case $phase in caps*) expected=$caps ;; *) expected=$notify ;; esac
    count=0
    [ ! -f "$dir/$phase" ] || count=$(wc -l <"$dir/$phase" | tr -d ' ')
    if [ "$count" != "$expected" ]; then
      echo "FAIL fatal $mode $phase=$count expected=$expected"; tail -n 25 "$work/portal.log"; exit 1
    fi
  done
  [ "$mode" != ack ] || [ -f "$dir/ack" ]
  [ "$mode" != auth-timeout ] || [ -f "$work/auth-accepted" ]
  echo "OK   fatal $mode: zero returns, diagnostic policy and bounded private IPC"
}
for mode in plain markup numeric nul after-nul invalid-title invalid-message writefail ack \
  caps-error notify-error caps-timeout notify-timeout deadline absent deadbus auth-timeout \
  failed-init independent lazy-runtime bootstrap; do
  fatal "$mode"
done
