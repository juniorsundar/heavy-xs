#!/usr/bin/env sh
# Builds the hook-enabled contract executable, stages it, runs each contract suite as a fresh
# bounded process in a runner-owned environment, and diffs it against its recorded golden.
#   scripts/rust-port/verify.sh [suite,suite,...]   verify (default: every suite with a golden)
# Environment:
#   PROFILE       xvfb (default; native windows require the runner-owned private X11 display)
#   CARGO_ARGS    extra cargo flags, e.g. "--release"
#   TARGET_DIR    cargo target dir (default target/contract; the developer's target/debug is left alone)
#   STAGE_DIR     staged install (default $TARGET_DIR/stage)
#   BIN           run this prebuilt editor instead of building one; STAGE_DIR must already hold its
#                 staged data (scripts/rust-port/sdl-reference.sh uses this for the pinned reference);
#                 BIN_FEATURES names its cargo features for the provenance record
#   EVIDENCE_DIR  keep each suite's output, its backend and a provenance record here
#   RECORD=1      also copy each passing suite's output to $EVIDENCE_DIR/record for review. Goldens
#                 are never replaced by this script; that is a reviewed, manual copy.
#   SUITE_SIGINT  default (default) | ignore: the SIGINT disposition each suite process starts with
# The system suite also runs clipboard-check.sh invariants on a separate private Xvfb, regardless
# of PROFILE; its historical golden retains the unavailable-display clipboard results.
# sysplugin/syswindow also check real dialog/fatal transport on scripted private services.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/scripts/rust-port/private-display.sh"
profile=${PROFILE:-xvfb}
target=${TARGET_DIR:-$root/target/contract}
stage_root=${STAGE_DIR:-$target/stage}
stage=$stage_root/usr/local
suites=${1:-$(ls "$root/scripts/rust-port/baseline" | sed "s/\.txt$//" | tr "\n" "," | sed "s/,$//")}
case $profile in xvfb) expect_backend=x11 ;;
  *) echo "PROFILE must be xvfb for the native platform" >&2; exit 2 ;; esac

# Cargo reads .cargo/config.toml (CFLAGS for the vendored Lua) from the current directory.
cd "$root"
if [ -z "${BIN:-}" ]; then
  case " ${CARGO_ARGS:-} " in *" --release "*) build=release ;; *) build=debug ;; esac
  bin=$target/$build/lite-xl
  # Suites that inject events or read pixels need the `test_hooks` feature (src/test_hooks.rs).
  # This executable is the contract one only; normal-build-check.sh proves a normal build has none.
  # shellcheck disable=SC2086
  env -u LD_LIBRARY_PATH cargo build --locked --manifest-path "$root/Cargo.toml" --target-dir "$target" ${CARGO_ARGS:-} \
    --features test_hooks >&2
  # A release binary carries no "(git-xxxx)" suffix (build.rs), so its start.lua must not either.
  release=; [ "$build" = release ] && release=1
  LITE_RELEASE=$release "$root/scripts/install-linux.sh" "$bin" "$stage_root" /usr/local >/dev/null
  features=test_hooks
else
  bin=$BIN
  [ -x "$bin" ] && [ -d "$stage/share/lite-xl" ] || { echo "BIN needs STAGE_DIR with a staged install" >&2; exit 2; }
  features=${BIN_FEATURES:-unrecorded}
fi
# Install the harness next to the data dir so `require` finds it without touching anything in
# data/. The editor selects it with LITE_XL_RUNTIME.
share=$stage/share/lite-xl
mkdir -p "$share/contract_cases"
cp "$root/tests/contract/contract.lua" "$share/contract.lua"
cp "$root"/tests/contract/cases/*.lua "$share/contract_cases/"

work=$(mktemp -d)
cleanup() { pd_profile_stop; rm -rf "$work"; }
trap cleanup EXIT
trap 'exit 130' INT TERM
pd_profile_start "$profile" "$work/display"

evidence=${EVIDENCE_DIR:-}
if [ -n "$evidence" ]; then
  mkdir -p "$evidence"
  sum() { sha256sum "$@" | cut -d' ' -f1; }
  {
    echo "runner $(git -C "$root" rev-parse HEAD)$(git -C "$root" diff --quiet HEAD -- scripts tests || echo ' +uncommitted test changes')"
    echo "binary $bin sha256=$(sum "$bin") features=$features"
    pd_run sh -c 'LD_TRACE_LOADED_OBJECTS=1 "$1"' sh "$bin" | sed -n 's/^\t*\(libSDL3[^ ]*\) => \([^ ]*\).*/sdl_linked \1 => \2/p'
    pd_describe
    echo "cases sha256=$(cat "$root"/tests/contract/contract.lua "$root"/tests/contract/cases/*.lua | sum)"
    echo "plugin_fixtures sha256=$(find "$root/tests/contract/plugin" -type f -print0 | sort -z | xargs -0 cat | sum)"
    echo "fonts sha256=$(find "$share/fonts" -type f -print0 | sort -z | xargs -0 cat | sum)"
    echo "goldens sha256=$(cat "$root"/scripts/rust-port/baseline/*.txt | sum)"
  } >"$evidence/provenance.txt"
fi

# SDL's "system" log category is enabled only to observe the video backend it really chose; these
# exact lines are SDL's and are taken out again before the diff. Anything else SDL or a driver
# prints stays in the output and must match (or be reviewed).
sdl_lines="^(App name|App version|App ID|SDL revision): |^SDL chose (video|render) backend '[^']*'\$"
# Mesa's notice when SDL's X11 window framebuffer starts its Vulkan renderer on Xvfb: driver
# output of the SDL reference, reviewed and dropped (docs/wayfinder/assets/sdl3-step1-evidence.md).
sdl_lines="$sdl_lines|^MESA: info: vulkan: No DRI3 support detected - required for presentation\$"
sdl_lines="$sdl_lines|^Note: you can probably enable DRI3 in your Xorg config\$"
sdl_lines="$sdl_lines|^Lite XL native backend '[^' ]*'\$"

# Preserve the system suite's historical unavailable-clipboard observations. Actual clipboard
# and primary-selection round trips are asserted separately by clipboard-check.sh on private Xvfb.
run_suite() {
  if [ "$s" = system ]; then
    pd_run env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET "$@"
  else
    pd_run "$@"
  fi
}

status=0
for s in $(echo "$suites" | tr , ' '); do
  raw=$work/$s.raw out=$work/$s.out
  # A crash or non-zero exit must fail the run, not silently truncate the diff. SIGINT starts at
  # its default: children inherit an ignored SIGINT through exec, and the process suite's
  # "interrupt then dead" then fails (reproduced on the SDL reference; see sdl-reference.sh).
  if run_suite SDL_LOGGING=system=debug \
      LXL_PLUGIN_SRC="$root/tests/contract/plugin" LXL_PLUGIN_INC="$root/resources/include" \
      CONTRACT_SUITES="$s" LITE_PREFIX="$stage" LITE_XL_RUNTIME=contract \
      env --"${SUITE_SIGINT:-default}"-signal=INT timeout -k 2 60 "$bin" >"$raw" 2>&1; then
    rc=0
  else
    rc=$?
    echo "exit status $rc" >>"$raw"
  fi
  backend=$(sed -n -e "s/^SDL chose video backend '\(.*\)'\$/\1/p" -e "s/^Lite XL native backend '\(.*\)'\$/\1/p" "$raw" | sort -u | tr '\n' ' ' | sed 's/ $//')
  LC_ALL=C grep -a -v -E "$sdl_lines" "$raw" >"$out" || true
  if [ -n "$evidence" ]; then
    cp "$out" "$evidence/$s.txt"
    echo "$s exit=$rc backend=${backend:-none}" >>"$evidence/backends.txt"
  fi
  base=$root/scripts/rust-port/baseline/$s.txt
  # No video init at all is fine (nothing could leak); any other backend than the profile's is not.
  if [ -n "$backend" ] && [ "$backend" != "$expect_backend" ]; then
    echo "FAIL $s (video backend '$backend', expected '$expect_backend')"; status=1
  elif [ "$rc" != 0 ]; then
    echo "FAIL $s (exit $rc)"; sed -n '$p' "$out"; status=1
  elif diff -u "$base" "$out"; then
    echo "OK   $s${backend:+ [$backend]}"
  else
    echo "FAIL $s${backend:+ [$backend]}"; status=1
  fi
  # A failed or crashed suite never reaches the review area, let alone a golden.
  if [ "${RECORD:-}" = 1 ] && [ "$rc" = 0 ] && { [ -z "$backend" ] || [ "$backend" = "$expect_backend" ]; }; then
    [ -n "$evidence" ] || { echo "RECORD=1 needs EVIDENCE_DIR" >&2; exit 2; }
    mkdir -p "$evidence/record"
    cp "$out" "$evidence/record/$s.txt"
  fi
done
case ",$suites," in
  *,system,*)
    BIN="$bin" STAGE_DIR="$stage_root" "$root/scripts/rust-port/clipboard-check.sh" || status=1 ;;
esac
case ",$suites," in
  *,sysplugin,*|*,syswindow,*)
    BIN="$bin" STAGE_DIR="$stage_root" TARGET_DIR="$target" "$root/scripts/rust-port/dialog-check.sh" || status=1 ;;
esac
[ "${RECORD:-}" = 1 ] && echo "recorded outputs for review in $evidence/record (provenance.txt names the binary)"
exit $status
