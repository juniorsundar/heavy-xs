#!/usr/bin/env bash
# PROTOTYPE SDL cost baseline driver. Runs every workload for builds A and B in A B B A order
# over ROUNDS rounds, so A-vs-B shows run-to-run noise when both are the same build.
# Usage: cost-run.sh OUTDIR
# Env: LXL_A, LXL_B (staged prefixes, default this worktree's stage), CONFIGS ("bundled user"),
#      ROUNDS (3), BACKEND (wayland | x11 | offscreen for smoke tests only),
#      MODES ("startup frames idle_focused idle_unfocused memory"),
#      STARTUP_N (10), IDLE_S (20), FRAME_N (600), MEM_CYCLES (50).
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$here/../../../.." && pwd)
out=${1:?OUTDIR required}
mkdir -p "$out"
stage=$root/target/stage/usr/local
LXL_A=${LXL_A:-$stage}
LXL_B=${LXL_B:-$stage}
ROUNDS=${ROUNDS:-3}
CONFIGS=${CONFIGS:-"bundled user"}
MODES=${MODES:-"startup frames idle_focused idle_unfocused memory"}
STARTUP_N=${STARTUP_N:-10}
BACKEND=${BACKEND:-wayland}
export IDLE_S=${IDLE_S:-20} FRAME_N=${FRAME_N:-600} MEM_CYCLES=${MEM_CYCLES:-50}

case $BACKEND in
  wayland|x11) ;;
  offscreen) ;;  # smoke tests only: no real presentation, no focus
  *) echo "BACKEND must be wayland, x11 or offscreen" >&2; exit 2 ;;
esac

# Per-launch limit: about twice the expected run, so a hang fails fast instead of stalling.
limit_for() {
  case $1 in
    startup) echo 30 ;;
    frames) echo $(( 30 + FRAME_N * 4 / 60 * 2 )) ;;
    idle_*) echo $(( IDLE_S + 30 )) ;;
    memory) echo $(( 30 + MEM_CYCLES )) ;;
  esac
}

# Plain-text sample so no language server attaches (the user config configures several).
sample=$out/sample.txt
[[ -f $sample ]] || cp "$root/src/renderer.rs" "$sample"

make_userdir() { # $1 = bundled|user
  local dir=$out/userdir-$1
  rm -rf "$dir"; mkdir -p "$dir"
  if [[ $1 == user ]]; then
    cp -r "$HOME/.config/lite-xl/." "$dir/"
    # Plugins in the user's config still declare an older mod-version.
    printf '\nconfig.skip_plugins_version = true\n' >> "$dir/init.lua"
  else
    printf 'local core = require "core"\n' > "$dir/init.lua"
  fi
  cat "$here/cost-probe.lua" >> "$dir/init.lua"
  # Start maximised from the session, as a user's maximised editor does, and drop recent projects.
  printf 'return {recents={}, window_mode="maximized", previous_find={}, previous_replace={}}\n' > "$dir/session.lua"
  echo "$dir"
}

# utime+stime ticks and context switches of a running pid, from /proc.
proc_sample() { # $1=pid -> "ticks vol invol"
  local stat ticks vol invol
  stat=$(cat "/proc/$1/stat") || return 1
  stat=${stat##*) }
  read -r -a f <<< "$stat"
  ticks=$(( f[11] + f[12] ))
  vol=$(awk '/^voluntary_ctxt_switches/{print $2}' "/proc/$1/status")
  invol=$(awk '/^nonvoluntary_ctxt_switches/{print $2}' "/proc/$1/status")
  echo "$ticks $vol $invol"
}

# Unfocused idle: the loop blocks in wait_event() with no timeout, so it is measured from outside.
# The probe writes its own pid into the ready file, since lite-xl runs under env and timeout.
run_unfocused() { # $1=tsv
  local tsv=$1 pid="" focus wmode i
  for i in $(seq 1 300); do [[ -s $tsv.ready ]] && break; sleep 0.1; done
  [[ -s $tsv.ready ]] || return 1
  read -r pid focus wmode < "$tsv.ready"
  [[ -d /proc/$pid ]] || return 1
  local a b t0 t1
  t0=$(date +%s%N); read -r -a a <<< "$(proc_sample "$pid")"
  sleep "$IDLE_S"
  t1=$(date +%s%N); read -r -a b <<< "$(proc_sample "$pid")"
  local el; el=$(awk -v a="$t0" -v b="$t1" 'BEGIN{printf "%.3f", (b-a)/1e9}')
  local cpu=$(( (b[0] - a[0]) * 10 ))
  {
    # focused_start/end hold the pre-minimise focus and the requested mode; the editor is not
    # stepped after minimising, so it cannot report later state itself.
    printf 'elapsed_s\tcpu_ms\tcpu_pct\tframes\twait_calls\twait_timeouts\twait_sub_ms\tsleep_calls\tvol_ctxsw\tinvol_ctxsw\tfocused_start\tfocused_end\n'
    printf '%s\t%d\t%s\tNA\tNA\tNA\tNA\tNA\t%d\t%d\tpre-minimise:%s\tmode:%s\n' "$el" "$cpu" \
      "$(awk -v c="$cpu" -v e="$el" 'BEGIN{printf "%.3f", c/(e*10)}')" \
      $(( b[1] - a[1] )) $(( b[2] - a[2] )) "$focus" "$wmode"
  } > "$tsv"
  echo "mode=idle_unfocused measured=external" > "$tsv.meta"
  kill "$pid" 2>/dev/null; sleep 0.5; kill -9 "$pid" 2>/dev/null || true
}

run_one() { # $1=label $2=prefix $3=config $4=mode $5=round $6=index
  local label=$1 prefix=$2 cfg=$3 mode=$4 round=$5 idx=$6
  local userdir; userdir=$(make_userdir "$cfg")
  local tsv=$out/$mode-$cfg-$label-r$round-$idx.tsv
  local rc=0 t0
  rm -f "$tsv" "$tsv.ready" "$tsv.meta"
  t0=$(date +%s%N)
  env SDL_VIDEODRIVER="$BACKEND" COST_MODE="$mode" COST_OUT="$tsv" COST_FILE="$sample" COST_T0_NS="$t0" \
    LITE_USERDIR="$userdir" LITE_PREFIX="$prefix" \
    timeout -k 2 "$(limit_for "$mode")" "$prefix/bin/lite-xl" "$sample" > "$tsv.log" 2>&1 &
  local tpid=$!
  if [[ $mode == idle_unfocused ]]; then
    run_unfocused "$tsv" || echo "unfocused sampling failed" >> "$tsv.log"
  fi
  wait "$tpid" || rc=$?
  # The unfocused editor is ended by the driver, so its exit status is not a failure.
  [[ $mode == idle_unfocused ]] && rc=0
  if (( rc != 0 )) || [[ ! -s $tsv ]]; then
    echo "FAILED $(basename "$tsv") exit=$rc" | tee -a "$out/failures.txt"
  fi
}

for round in $(seq 1 "$ROUNDS"); do
  for cfg in $CONFIGS; do
    for mode in $MODES; do
      n=1; [[ $mode == startup ]] && n=$((STARTUP_N + 1))  # first startup run is discarded later
      for pair in "A:$LXL_A" "B:$LXL_B" "B:$LXL_B" "A:$LXL_A"; do
        label=${pair%%:*}; prefix=${pair#*:}
        for _ in $(seq 1 "$n"); do
          k=$(find "$out" -maxdepth 1 -name "$mode-$cfg-$label-r$round-*.tsv.log" | wc -l)
          run_one "$label" "$prefix" "$cfg" "$mode" "$round" "$k"
        done
      done
      echo "round $round $cfg $mode done"
    done
  done
done
