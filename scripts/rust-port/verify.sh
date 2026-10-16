#!/usr/bin/env sh
# Build, stage, run the headless contract runtime, and diff against the recorded C baseline.
#   scripts/rust-port/verify.sh [suite,suite,...]   verify (default: smoke)
#   RECORD=1 scripts/rust-port/verify.sh suite      (re)record the baseline for each suite
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
build=${BUILD_DIR:-$root/build}
stage=$build/stage/usr/local
suites=${1:-smoke}

ninja -C "$build" >/dev/null 2>&1 || { ninja -C "$build"; exit 1; }
meson install -C "$build" --destdir "$build/stage" >/dev/null 2>&1

# Install harness next to the data dir so `require` finds it without touching data/.
share=$stage/share/lite-xl
mkdir -p "$share/contract_cases"
cp "$root/tests/contract/contract.lua" "$share/contract.lua"
cp "$root"/tests/contract/cases/*.lua "$share/contract_cases/"

status=0
for s in $(echo "$suites" | tr , ' '); do
  out=$(mktemp)
  SDL_VIDEODRIVER=${SDL_VIDEODRIVER:-offscreen} LITE_PREFIX=$stage LITE_XL_RUNTIME=contract \
    CONTRACT_SUITES=$s timeout 60 "$build/src/lite-xl" >"$out" 2>&1 || true
  base=$root/scripts/rust-port/baseline/$s.txt
  if [ "${RECORD:-}" = 1 ]; then
    cp "$out" "$base"; echo "recorded $base"
  elif diff -u "$base" "$out"; then
    echo "OK   $s"
  else
    echo "FAIL $s"; status=1
  fi
  rm -f "$out"
done
exit $status
