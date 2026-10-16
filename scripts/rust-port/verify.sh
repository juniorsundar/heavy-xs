#!/usr/bin/env sh
# Build with Cargo, stage, run the headless contract runtime, and diff against the recorded baseline.
#   scripts/rust-port/verify.sh [suite,suite,...]   verify (default: every suite with a baseline)
#   RECORD=1 scripts/rust-port/verify.sh suite      (re)record the baseline for each suite
# Environment: CARGO_ARGS (extra cargo flags, e.g. "--release" or "--features sdl_renderer"),
# TARGET_DIR (cargo target dir; default rust/target, use another one per configuration).
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
target=${TARGET_DIR:-$root/rust/target}
case " ${CARGO_ARGS:-} " in *" --release "*) profile=release ;; *) profile=debug ;; esac
bin=$target/$profile/lite-xl
stage=${STAGE_DIR:-$target/stage}/usr/local
suites=${1:-$(ls "$root/scripts/rust-port/baseline" 2>/dev/null | sed "s/\.txt$//" | tr "\n" "," | sed "s/,$//")}

# Suites that inject synthetic SDL events need the Rust `test_hooks` feature
# (rust/src/test_hooks.rs). It is enabled only for those runs, so a normal build never ships it.
needs_hooks=0
for s in $(echo "$suites" | tr , ' '); do
  case $s in sysevents|syswindow|sysplugin|customevents|rencache|renderer|restart) needs_hooks=1 ;; esac
done
features=
[ "$needs_hooks" = 1 ] && features="--features test_hooks"

# shellcheck disable=SC2086
cargo build --manifest-path "$root/rust/Cargo.toml" --target-dir "$target" ${CARGO_ARGS:-} $features >&2

# A release binary carries no "(git-xxxx)" suffix (rust/build.rs), so its start.lua must not either.
release=; [ "$profile" = release ] && release=1
LITE_RELEASE=$release "$root/scripts/install-linux.sh" "$bin" "$target/stage" /usr/local >/dev/null
# Install the harness next to the data dir so `require` finds it without
# touching anything in data/. The editor selects it with LITE_XL_RUNTIME.
share=$stage/share/lite-xl
mkdir -p "$share/contract_cases"
cp "$root/tests/contract/contract.lua" "$share/contract.lua"
cp "$root"/tests/contract/cases/*.lua "$share/contract_cases/"

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
      timeout -k 2 60 "$bin" >"$out" 2>&1; then
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
