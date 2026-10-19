#!/usr/bin/env bash
# THROWAWAY Wayland presentation diagnostic. Does not capture the screen or modify the desktop.
# Logs can contain what you type and Wayland protocol contents: use throwaway text and
# review the output directory before sharing it.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
usage() {
  cat <<'EOF'
Usage: presentation-check.sh --run zero|ff OUTDIR

Without --run this script is a no-op and only prints this usage. --help also
prints this usage without building or launching anything.

--run starts exactly one Wayland probe case with --smoke-seconds 8. Run it only
with the user's permission and leave the desktop idle. It refuses a non-Wayland
session or an unset WAYLAND_DISPLAY and never fakes session variables. It takes
no screen capture. SSH sessions are refused even if Wayland variables are inherited.

OUTDIR must not exist yet. It receives metadata.txt and live.log, plus
wayland-debug.log when WAYLAND_DEBUG=client is set. These logs may contain
private keyboard text and protocol data.

zero: softbuffer's documented zero high byte (default reproduction)
ff:   Wayland-only opaque high-byte diagnostic workaround
The probe window title names the padding mode; the drawn scene is identical.
Exit 0 means at least one frame was presented, not that it is visible or correct.
EOF
}
if [[ $# -eq 0 ]]; then usage; exit 0; fi
case $1 in --help|-h) usage; exit 0;; esac
if [[ $1 != --run ]]; then usage >&2; exit 2; fi
if [[ $# -ne 3 ]]; then
  echo 'presentation-check: --run needs MODE and OUTDIR' >&2
  usage >&2
  exit 2
fi
mode=$2
outdir=$3
case $mode in zero|ff) ;; *) echo "presentation-check: mode must be zero|ff, got $mode" >&2; exit 2;; esac
if [[ -z $outdir ]]; then echo 'presentation-check: OUTDIR must be non-empty' >&2; exit 2; fi
if [[ -e $outdir ]]; then echo "presentation-check: OUTDIR already exists; refusing to overwrite: $outdir" >&2; exit 2; fi
if [[ ${XDG_SESSION_TYPE:-} != wayland ]]; then
  echo "presentation-check: not a Wayland session (XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unset}); refusing" >&2
  exit 2
fi
if [[ -z ${WAYLAND_DISPLAY:-} ]]; then
  echo 'presentation-check: WAYLAND_DISPLAY is unset; refusing' >&2
  exit 2
fi
if [[ -n ${SSH_CONNECTION:-}${SSH_CLIENT:-}${SSH_TTY:-} ]]; then
  echo 'presentation-check: SSH session; run locally at the desktop instead' >&2
  exit 2
fi
root=$(cd "$here/../../../.." && pwd)
mkdir -- "$outdir"
{
  printf 'date_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'commit=%s\n' "$(git -C "$root" rev-parse HEAD 2>/dev/null || echo unknown)"
  printf 'branch=%s\n' "$(git -C "$root" branch --show-current 2>/dev/null || echo unknown)"
  printf 'kernel=%s\n' "$(uname -srm)"
  printf 'cargo=%s\n' "$(cargo --version 2>/dev/null || echo unavailable)"
  printf 'rustc=%s\n' "$(rustc --version 2>/dev/null || echo unavailable)"
  printf 'mode=%s\n' "$mode"
  printf 'XDG_SESSION_TYPE=%s\n' "$XDG_SESSION_TYPE"
  printf 'WAYLAND_DISPLAY=%s\n' "$WAYLAND_DISPLAY"
  printf 'DISPLAY_set=%s\n' "${DISPLAY:+yes}"
  printf 'WAYLAND_DEBUG=%s\n' "${WAYLAND_DEBUG:-unset}"
  printf 'PROBE_NIX=%s\n' "${PROBE_NIX:-unset}"
  printf 'PROBE_FONT_set=%s\n' "${PROBE_FONT:+yes}"
} > "$outdir/metadata.txt"
set +e
if [[ ${WAYLAND_DEBUG:-} == client ]]; then
  # Kept separate because protocol logging is verbose and may include private input.
  bash "$here/run.sh" --backend wayland --wayland-padding "$mode" --smoke-seconds 8 \
    > "$outdir/live.log" 2> "$outdir/wayland-debug.log"
else
  bash "$here/run.sh" --backend wayland --wayland-padding "$mode" --smoke-seconds 8 \
    > "$outdir/live.log" 2>&1
fi
status=$?
set -e
printf 'exit=%s\n' "$status" >> "$outdir/metadata.txt"
if [[ $status -eq 0 ]]; then
  echo "presentation-check: exit 0: at least one frame was presented, not proof of a visible or correct window."
  echo "Record what you observed during the eight-second run using the checklist in PRESENTATION.md."
else
  echo "presentation-check: exit $status; inspect $outdir/live.log"
  if [[ ${WAYLAND_DEBUG:-} == client ]]; then echo "presentation-check: protocol log: $outdir/wayland-debug.log"; fi
fi
echo "presentation-check: logs: $outdir (may contain private input)"
exit "$status"
