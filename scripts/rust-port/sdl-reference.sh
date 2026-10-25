#!/usr/bin/env sh
# Builds the pinned SDL reference editor apart from the developer's build and install, and
# collects the evidence later SDL-free candidates are compared with (see
# docs/architecture/wayfinder/assets/sdl3-migration-handoff.md, Step 1).
#   scripts/rust-port/sdl-reference.sh [build|probe|contract|all]     (default all)
# Environment:
#   REF_DIR          everything the reference owns (default target/sdl3-reference):
#                    source/ (detached git worktree), target/, stage/, probe/, evidence/
#   REF_COMMIT       pinned source (default the handoff's implementation reference)
#   REF_LOCK_SHA256  required Cargo.lock digest of that source
#   PROCESS_REPEAT   runs of the process suite per profile when classifying its timing case (20)
#   RECORD=1         write probe outputs that differ from scripts/rust-port/sdl-expectations to
#                    $REF_DIR/evidence/record for review; the checked-in expectations are never
#                    replaced by this script
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/scripts/rust-port/private-display.sh"
# The reference is an SDL editor: select its drivers in every private run, including verify.sh's.
export PD_SDL=1
commit=${REF_COMMIT:-9c1b13db14d20901004b6630fce19812c25e9cd9}
lock_sha=${REF_LOCK_SHA256:-1a2b1485592910d3bc588c04d8a7f12f0d285b9997a6cab91eac66a718f72a35}
ref=${REF_DIR:-$root/target/sdl3-reference}
src=$ref/source
bin=$ref/target/debug/lite-xl
evidence=$ref/evidence
expect=$root/scripts/rust-port/sdl-expectations
mkdir -p "$evidence"

work=$(mktemp -d)
cleanup() { pd_profile_stop; rm -rf "$work"; }
trap cleanup EXIT
trap 'exit 130' INT TERM

# Runs "$@" under PROFILE in its own private environment (and Xvfb, if any).
in_profile() {
  ip_profile=$1; shift
  pd_profile_start "$ip_profile" "$work/display.$ip_profile"
  rc=0; pd_run "$@" || rc=$?
  pd_profile_stop
  return $rc
}

build() {
  if [ ! -e "$src/.git" ]; then
    git -C "$root" worktree add --detach "$src" "$commit" >&2
  fi
  head=$(git -C "$src" rev-parse HEAD)
  [ "$head" = "$commit" ] || { echo "STOP: $src is at $head, not $commit" >&2; exit 1; }
  [ -z "$(git -C "$src" status --porcelain)" ] || { echo "STOP: $src has local changes" >&2; exit 1; }
  got=$(sha256sum "$src/Cargo.lock" | cut -d' ' -f1)
  [ "$got" = "$lock_sha" ] || { echo "STOP: Cargo.lock sha256 $got, expected $lock_sha" >&2; exit 1; }
  # The contract executable: the same test_hooks feature verify.sh builds candidates with.
  (cd "$src" && env -u LD_LIBRARY_PATH cargo build --locked --features test_hooks --target-dir "$ref/target" >&2)
  rm -rf "$ref/stage"
  sh "$src/scripts/install-linux.sh" "$bin" "$ref/stage" /usr/local >/dev/null
  build_probe
  {
    echo "source $commit (detached worktree $src, clean)"
    echo "cargo_lock sha256=$got"
    echo "build profile=debug features=test_hooks --locked"
    echo "binary sha256=$(sha256sum "$bin" | cut -d' ' -f1)"
    echo "toolchain $(rustc -V) / $(cargo -V)"
    echo "sdl3_sys $(sed -n '/^name = "sdl3-sys"$/{n;s/^version = "\(.*\)"/\1/p;}' "$src/Cargo.lock") (bindings only)"
    in_profile offscreen sh -c 'LD_TRACE_LOADED_OBJECTS=1 "$1"' sh "$bin" |
      sed -n 's/^\t*\(libSDL3[^ ]*\) => \([^ ]*\).*/sdl_linked \1 => \2/p'
    in_profile offscreen "$ref/probe/sdl_reference_probe" version
    echo "probe_linked $(in_profile offscreen sh -c 'LD_TRACE_LOADED_OBJECTS=1 "$1"' sh "$ref/probe/sdl_reference_probe" |
      sed -n 's/^\t*libSDL3[^ ]* => \([^ ]*\).*/\1/p')"
    echo "stage $ref/stage/usr/local (installed by the reference's own install-linux.sh)"
    echo "fonts sha256=$(find "$ref/stage/usr/local/share/lite-xl/fonts" -type f -print0 | sort -z | xargs -0 cat | sha256sum | cut -d' ' -f1)"
  } >"$evidence/reference.txt"
  cat "$evidence/reference.txt"
}

build_probe() {
  mkdir -p "$ref/probe"
  # Without LD_LIBRARY_PATH, as for the editor: ld would resolve libSDL3's dependencies through it.
  # shellcheck disable=SC2046
  env -u LD_LIBRARY_PATH cc -O1 -Wall -DPROBE_CLICKS "$root/scripts/rust-port/fixtures/sdl_reference_probe.c" \
    $(pkg-config --cflags --libs sdl3 x11 xtst) -o "$ref/probe/sdl_reference_probe"
}

# Compares one observation with its checked-in expectation; a difference fails and, with RECORD=1,
# is left in the review area.
compare() {
  name=$1 got=$2
  if [ -f "$expect/$name" ] && diff -u "$expect/$name" "$got"; then
    echo "OK   $name"
  else
    echo "FAIL $name (differs from scripts/rust-port/sdl-expectations/$name)"
    if [ "${RECORD:-}" = 1 ]; then mkdir -p "$evidence/record"; cp "$got" "$evidence/record/$name"; fi
    return 1
  fi
}

probe() {
  [ -x "$ref/probe/sdl_reference_probe" ] || build_probe
  p=$ref/probe/sdl_reference_probe
  status=0
  for profile in offscreen xvfb; do
    in_profile "$profile" timeout -k 2 60 "$p" draw >"$evidence/probe-draw-$profile.raw" 2>"$evidence/probe-draw-$profile.err" ||
      { echo "FAIL probe draw ($profile)"; status=1; continue; }
    head -1 "$evidence/probe-draw-$profile.raw"
    sed 1d "$evidence/probe-draw-$profile.raw" >"$work/draw.txt"
    compare draw.txt "$work/draw.txt" || status=1
  done
  # Real X button events through XTest, so only the private Xvfb can produce them.
  in_profile xvfb timeout -k 2 120 "$p" clicks >"$evidence/probe-clicks-xvfb.raw" 2>"$evidence/probe-clicks-xvfb.err" ||
    { echo "FAIL probe clicks"; status=1; }
  grep -v -E '^(reset|settle) ' "$evidence/probe-clicks-xvfb.raw" | sed 1d >"$work/clicks.txt"
  compare clicks.txt "$work/clicks.txt" || status=1
  return $status
}

contract() {
  status=0
  for profile in offscreen xvfb; do
    echo "== contract suites, $profile"
    BIN=$bin BIN_FEATURES=test_hooks STAGE_DIR=$ref/stage PROFILE=$profile \
      EVIDENCE_DIR=$evidence/contract-$profile sh "$root/scripts/rust-port/verify.sh" \
      >"$evidence/contract-$profile.log" 2>&1 || status=1
    grep -E '^(OK|FAIL) ' "$evidence/contract-$profile.log"
  done
  for profile in dummy xvfb; do
    echo "== startup checks, $profile"
    BIN=$bin STAGE_DIR=$ref/stage STARTUP_PROFILE=$profile sh "$root/scripts/rust-port/startup-check.sh" \
      >"$evidence/startup-$profile.log" 2>&1 || status=1
    cat "$evidence/startup-$profile.log"
  done
  echo "== hit-test, SDL comparator and recorded expectation"
  HIT_TEST_REFERENCE=c sh "$root/scripts/rust-port/hit-test-check.sh" >"$evidence/hit-test.log" 2>&1 || status=1
  cat "$evidence/hit-test.log"
  # The process suite's timing case: repeat it so a failure is reproduced and classified, never
  # mistaken for a candidate regression or waived.
  n=${PROCESS_REPEAT:-20}
  for profile in offscreen xvfb; do
    : >"$evidence/process-repeat-$profile.txt"
    i=1
    while [ "$i" -le "$n" ]; do
      if BIN=$bin BIN_FEATURES=test_hooks STAGE_DIR=$ref/stage PROFILE=$profile \
          EVIDENCE_DIR=$work/process-$profile-$i sh "$root/scripts/rust-port/verify.sh" process >"$work/p.log" 2>&1; then
        echo "run $i OK" >>"$evidence/process-repeat-$profile.txt"
      else
        echo "run $i FAIL: $(grep -E '^[-+][^-+]' "$work/p.log" | tr '\n' ' ')" >>"$evidence/process-repeat-$profile.txt"
      fi
      i=$((i + 1))
    done
    echo "== process x$n, $profile: $(grep -c ' OK$' "$evidence/process-repeat-$profile.txt") passed"
    grep FAIL "$evidence/process-repeat-$profile.txt" || true
  done
  # The recorded failure's cause: an editor started with SIGINT ignored passes that to its
  # children, so interrupt() cannot end them. Kept as a classified observation, not a pass.
  BIN=$bin BIN_FEATURES=test_hooks STAGE_DIR=$ref/stage PROFILE=offscreen SUITE_SIGINT=ignore \
    sh "$root/scripts/rust-port/verify.sh" process >"$evidence/process-sigint-ignored.log" 2>&1 || true
  echo "== process with SIGINT inherited as ignored (classification, expected to differ):"
  grep -E '^[-+][^-+]|^(OK|FAIL)' "$evidence/process-sigint-ignored.log" || true
  return $status
}

case ${1:-all} in
  build) build ;;
  probe) probe ;;
  contract) contract ;;
  all) build; s=0; probe || s=1; contract || s=1; exit $s ;;
  *) echo "usage: $0 [build|probe|contract|all]" >&2; exit 2 ;;
esac
