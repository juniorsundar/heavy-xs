# Sourced by the rust-port checks: the runner-owned environments a test editor may run in.
#   pd_profile_start PROFILE WORKDIR   offscreen|dummy|xvfb; xvfb starts a private Xvfb
#   pd_run CMD...                      runs CMD in the isolated environment of that profile
#                                      (set pd_exec=1 in a subshell to exec it instead)
#   pd_profile_stop                    stops only the Xvfb this file started
# Nothing here reaches the developer's session: no inherited DISPLAY, Wayland socket, runtime
# dir or library path, and the session bus points at a path that does not exist (no portal,
# chooser, clipboard manager or notification daemon). Xvfb gets its own display number, cookie
# and runtime dir.

pd_profile=
pd_dir=
pd_pid=
pd_display=

pd_fail() { echo "private-display: $*" >&2; return 1; }

pd_profile_start() {
  pd_profile=$1
  pd_dir=$2
  mkdir -p "$pd_dir/run"
  chmod 700 "$pd_dir/run"
  case $pd_profile in
    offscreen | dummy) return 0 ;;
    xvfb) ;;
    *) pd_fail "unknown profile '$pd_profile' (offscreen, dummy or xvfb)"; return 1 ;;
  esac
  command -v Xvfb >/dev/null && command -v xauth >/dev/null && command -v mcookie >/dev/null ||
    { pd_fail "xvfb profile needs Xvfb, xauth and mcookie (nix develop provides them)"; return 1; }

  # Pick a display nobody holds; Xvfb itself refuses a taken one, so a lost race only retries.
  n=${PD_FIRST_DISPLAY:-90}
  tries=0
  while [ $tries -lt 20 ]; do
    if [ -e "/tmp/.X$n-lock" ] || [ -e "/tmp/.X11-unix/X$n" ]; then
      n=$((n + 1)); tries=$((tries + 1)); continue
    fi
    auth=$pd_dir/Xauthority
    : >"$auth"
    xauth -q -f "$auth" add ":$n" . "$(mcookie)" || { pd_fail "xauth failed"; return 1; }
    ready=$pd_dir/ready.$n
    : >"$ready"
    # -nolisten tcp: local socket only. -noreset keeps one server across the suite processes.
    Xvfb ":$n" -auth "$auth" -nolisten tcp -noreset -screen 0 "${PD_SCREEN:-1280x800x24}" \
      -dpi 96 -displayfd 3 3>"$ready" >"$pd_dir/Xvfb.$n.log" 2>&1 &
    pid=$!
    # Publish the owned PID before waiting, so an interrupt trap can stop it.
    pd_pid=$pid
    i=0
    while [ $i -lt "${PD_READY_TENTHS:-100}" ]; do
      [ -s "$ready" ] && break
      kill -0 "$pid" 2>/dev/null || break
      sleep 0.1; i=$((i + 1))
    done
    if [ -s "$ready" ] && kill -0 "$pid" 2>/dev/null; then
      pd_display=:$n
      return 0
    fi
    # Not ready: stop the one we started (never anything else) and try the next number.
    pd_profile_stop
    n=$((n + 1)); tries=$((tries + 1))
  done
  pd_fail "no private Xvfb became ready (see $pd_dir/Xvfb.*.log)"
}

pd_profile_stop() {
  # Callers run under set -e, often from an EXIT trap: a server that is already gone is no error.
  if [ -n "$pd_pid" ]; then
    kill "$pd_pid" 2>/dev/null || true
    i=0; while [ $i -lt 30 ] && kill -0 "$pd_pid" 2>/dev/null; do sleep 0.1; i=$((i + 1)); done
    kill -KILL "$pd_pid" 2>/dev/null || true
    wait "$pd_pid" 2>/dev/null || true
  fi
  pd_pid=
}

pd_run() {
  case $pd_profile in
    xvfb) set -- DISPLAY="$pd_display" XAUTHORITY="$pd_dir/Xauthority" SDL_VIDEO_DRIVER=x11 SDL_VIDEODRIVER=x11 "$@" ;;
    *) set -- SDL_VIDEO_DRIVER="$pd_profile" SDL_VIDEODRIVER="$pd_profile" "$@" ;;
  esac
  # The binary's own runpath picks its libraries: a shell's LD_LIBRARY_PATH can point at a build
  # for another glibc (seen: an alsa-lib needing GLIBC_2.43 under a 2.42 loader).
  ${pd_exec:+exec} env -u LD_LIBRARY_PATH -u LD_PRELOAD -u DISPLAY -u XAUTHORITY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u XDG_SESSION_TYPE \
    -u IBUS_ADDRESS -u XMODIFIERS -u GTK_IM_MODULE -u QT_IM_MODULE -u SDL_IM_MODULE \
    XDG_RUNTIME_DIR="$pd_dir/run" DBUS_SESSION_BUS_ADDRESS="unix:path=$pd_dir/no-session-bus" \
    SDL_FILE_DIALOG_DRIVER= "$@"
}

# One line describing the environment, for provenance records.
pd_describe() {
  case $pd_profile in
    xvfb) echo "profile=xvfb display=$pd_display screen=${PD_SCREEN:-1280x800x24} dpi=96 xvfb=$(command -v Xvfb) bus=unreachable runtime_dir=private" ;;
    *) echo "profile=$pd_profile display=none bus=unreachable runtime_dir=private" ;;
  esac
}
