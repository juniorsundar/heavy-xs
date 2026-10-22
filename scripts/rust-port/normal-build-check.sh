#!/usr/bin/env sh
# Builds the ordinary configuration (no test_hooks) apart from the contract executable and checks
# that nothing test-only ships in it: no `lxl_test` global or hook code, and no environment
# variable that switches implementations.
#   NORMAL_TARGET_DIR  cargo target dir (default target/normal)
#   CARGO_ARGS         extra cargo flags, e.g. "--release"
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/scripts/rust-port/private-display.sh"
target=${NORMAL_TARGET_DIR:-$root/target/normal}
case " ${CARGO_ARGS:-} " in *" --release "*) build=release ;; *) build=debug ;; esac
bin=$target/$build/lite-xl
stage=$target/stage/usr/local
share=$stage/share/lite-xl

cd "$root"
# shellcheck disable=SC2086
env -u LD_LIBRARY_PATH cargo build --locked --manifest-path "$root/Cargo.toml" --target-dir "$target" ${CARGO_ARGS:-} >&2
release=; [ "$build" = release ] && release=1
LITE_RELEASE=$release "$root/scripts/install-linux.sh" "$bin" "$target/stage" /usr/local >/dev/null

work=$(mktemp -d)
cleanup() { pd_profile_stop; rm -rf "$work" "$share/normal_probe.lua"; }
trap cleanup EXIT
trap 'exit 130' INT TERM
pd_profile_start offscreen "$work/display"
fail=0
check() { if [ "$2" = "$3" ]; then echo "OK   $1"; else echo "FAIL $1: expected [$3] got [$2]"; fail=1; fi; }

cat >"$share/normal_probe.lua" <<'LUA'
local M = {}
function M.init()
  local keys = {}
  for k in pairs(system) do keys[#keys + 1] = k end
  table.sort(keys)
  print("lxl_test", type(lxl_test), type(package.loaded.lxl_test))
  print("system", #keys, table.concat(keys, ","))
  os.exit(0)
end
function M.run() end
function M.on_error(e) print("error", e) os.exit(1) end
return M
LUA
run() { pd_run "$@" LITE_PREFIX="$stage" LITE_XL_RUNTIME=normal_probe timeout -k 2 30 "$bin" 2>&1; }
plain=$(run)
check "no lxl_test global or module" "$(echo "$plain" | grep '^lxl_test')" "lxl_test	nil	nil"
# The retired switch (and any other LXL_TEST_* name) must change nothing in a normal build.
check "LXL_TEST_C_EVENTS has no effect" "$(run LXL_TEST_C_EVENTS=1)" "$plain"
check "system exports the full table" "$(echo "$plain" | sed -n 's/^system	\([0-9]*\)	.*/\1/p')" 41
# Hook code is compiled out, not merely unregistered.
check "no hook symbols in the binary" "$(grep -c -a -E 'lxl_test|push_event|surface_hash|dirty_rects|DIALOG_CHECK_WORK|FATAL_CHECK_ROLE|LXL_PRIVATE_DISPLAY_TEST|LXL_FATAL_RUNTIME_TEST' "$bin" || true)" 0
exit $fail
