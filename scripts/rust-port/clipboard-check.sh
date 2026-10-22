#!/usr/bin/env sh
# Checks the native text clipboard and primary selection (src/clipboard.rs) end to end:
#   unavailable  no display at all: both gets return one empty string, writes are silent
#   owner/peer   two ordinary editor processes on one runner-owned private Xvfb; the peer reads
#                the owner's selections over the real X11 connection while the owner stays alive
# Runs the staged binary left behind by verify.sh (or BIN with its STAGE_DIR). No window is created
# and no test hook is used. The owner and peer share this script's private server, so do not run
# another clipboard check against it at the same time.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/scripts/rust-port/private-display.sh"
target=${TARGET_DIR:-$root/target/contract}
case " ${CARGO_ARGS:-} " in *" --release "*) profile=release ;; *) profile=debug ;; esac
bin=${BIN:-$target/$profile/lite-xl}
stage=${STAGE_DIR:-$target/stage}/usr/local
share=$stage/share/lite-xl
[ -x "$bin" ] && [ -d "$share" ] || { echo "run scripts/rust-port/verify.sh first"; exit 1; }

work=$(mktemp -d)
runtime=$share/clipboard_check.lua
owner_pid=
cleanup() {
  if [ -n "$owner_pid" ]; then
    kill "$owner_pid" 2>/dev/null || true
    wait "$owner_pid" 2>/dev/null || true
  fi
  pd_profile_stop
  rm -rf "$work" "$runtime"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

cp "$root/tests/contract/clipboard.lua" "$runtime"
fail=0
check() { if [ "$2" = "$3" ]; then echo "OK   $1"; else echo "FAIL $1: expected [$3] got [$2]"; fail=1; fi; }
dump() { if [ -f "$1" ]; then echo "--- $1"; sed 's/^/    /' "$1"; fi; }

# --- unavailable: offscreen profile, which unsets DISPLAY/Wayland and points the bus at nothing.
# The process must exit cleanly: a missing display is an ordinary empty clipboard, not an error.
pd_profile_start offscreen "$work/display-offscreen"
mkdir -p "$work/unavailable/user"
out=$work/unavailable.log
( cd "$work/unavailable" && pd_run CLIPBOARD_MODE=unavailable CLIPBOARD_CHECK_WORK="$work" \
    LITE_USERDIR="$work/unavailable/user" LITE_PREFIX="$stage" LITE_XL_RUNTIME=clipboard_check \
    timeout -k 2 30 "$bin" >"$out" 2>&1 ) && rc=0 || rc=$?
check "unavailable: exit code" "$rc" 0
check "unavailable: mode ran" "$(grep -c '^CLIPBOARD unavailable checks passed' "$out")" 1
if [ -e "$work/unavailable/error.txt" ] || [ -e "$work/unavailable/user/error.txt" ]; then
  check "unavailable: no error.txt" present absent
else
  check "unavailable: no error.txt" absent absent
fi
[ "$rc" = 0 ] || dump "$out"

# --- owner and peer: one private Xvfb, two fresh processes on it.
pd_profile_start xvfb "$work/display-xvfb"
pd_describe
mkdir -p "$work/user-owner" "$work/user-peer"
ready=$work/ready
done=$work/done
rm -f "$ready" "$done"

owner_out=$work/owner.log
# pd_exec replaces the subshell with timeout, so owner_pid is the bounded command we can stop.
( cd "$work" && pd_exec=1 && pd_run CLIPBOARD_MODE=owner CLIPBOARD_CHECK_WORK="$work" \
    LITE_USERDIR="$work/user-owner" LITE_PREFIX="$stage" LITE_XL_RUNTIME=clipboard_check \
    timeout -k 2 30 "$bin" >"$owner_out" 2>&1 ) &
owner_pid=$!
i=0
while [ $i -lt 100 ] && [ ! -e "$ready" ]; do
  kill -0 "$owner_pid" 2>/dev/null || break
  sleep 0.1
  i=$((i + 1))
done
check "owner: ready within 10s" "$([ -e "$ready" ] && echo yes || echo no)" yes

if [ -e "$ready" ]; then
  peer_out=$work/peer.log
  ( cd "$work" && pd_run CLIPBOARD_MODE=peer CLIPBOARD_CHECK_WORK="$work" \
      LITE_USERDIR="$work/user-peer" LITE_PREFIX="$stage" LITE_XL_RUNTIME=clipboard_check \
      timeout -k 2 30 "$bin" >"$peer_out" 2>&1 ) && prc=0 || prc=$?
  check "peer: exit code" "$prc" 0
  check "peer: mode ran" "$(grep -c '^CLIPBOARD peer read both selections' "$peer_out")" 1
  check "peer: wrote done" "$([ -e "$done" ] && echo yes || echo no)" yes
  [ "$prc" = 0 ] || dump "$peer_out"
else
  dump "$owner_out"
fi

# The owner has its own 30s timeout; it now sees done (or times out waiting for it).
wait "$owner_pid" && orc=0 || orc=$?
owner_pid=
check "owner: exit code" "$orc" 0
check "owner: restarted" "$(grep -c '^CLIPBOARD owner checks passed; restarting' "$owner_out")" 1
check "owner: completed" "$(grep -c '^CLIPBOARD owner and peer completed' "$owner_out")" 1
[ "$orc" = 0 ] || dump "$owner_out"

exit $fail
