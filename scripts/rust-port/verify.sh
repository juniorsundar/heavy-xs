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

status=0
for s in $(echo "$suites" | tr , ' '); do
  out=$(mktemp)
  # A crash or non-zero exit must fail the run, not silently truncate the diff.
  if CONTRACT_SUITES=$s SDL_VIDEODRIVER=${SDL_VIDEODRIVER:-offscreen} \
      LITE_PREFIX=$stage LITE_XL_RUNTIME=contract \
      timeout 60 "$build/src/lite-xl" >"$out" 2>&1; then
    rc=0
  else
    rc=$?
    echo "exit status $rc" >>"$out"
  fi
  base=$root/scripts/rust-port/baseline/$s.txt
  if [ "${RECORD:-}" = 1 ]; then
    cp "$out" "$base"
    echo "recorded $base (exit $rc)"
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
