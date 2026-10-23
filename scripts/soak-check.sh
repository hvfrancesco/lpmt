#!/usr/bin/env bash
# Run a timed release soak with process/GPU telemetry.
#
# Usage:
#   ./scripts/soak-check.sh
#   LPMT_SOAK_SECONDS=600 ./scripts/soak-check.sh
#   LPMT_SOAK_PROJECT=/path/show.json ./scripts/soak-check.sh
#   LPMT_SOAK_MAX_RSS_GROWTH_MB=64 LPMT_SOAK_MIN_FPS=55 ./scripts/soak-check.sh
set -euo pipefail
export LC_ALL=C

if [[ ! -f Cargo.toml ]]; then
    echo "run this script from the LPMT repository root" >&2
    exit 2
fi

seconds="${LPMT_SOAK_SECONDS:-1800}"
interval="${LPMT_SOAK_INTERVAL:-5}"
warmup="${LPMT_SOAK_WARMUP_SECONDS:-30}"
max_rss_growth_mb="${LPMT_SOAK_MAX_RSS_GROWTH_MB:-128}"
min_fps="${LPMT_SOAK_MIN_FPS:-0}"
features="${LPMT_FEATURES:-}"
profile="${LPMT_SOAK_PROFILE:-release}"
if [[ "$profile" == "release" ]]; then
    binary="target/release/lpmt-app"
    build_args=(--release)
elif [[ "$profile" == "debug" ]]; then
    binary="target/debug/lpmt-app"
    build_args=()
else
    echo "LPMT_SOAK_PROFILE must be release or debug" >&2
    exit 2
fi

if [[ "${LPMT_SOAK_SKIP_BUILD:-0}" != "1" ]]; then
    if [[ -n "$features" ]]; then
        cargo build "${build_args[@]}" -p lpmt-app --features "$features"
    else
        cargo build "${build_args[@]}" -p lpmt-app
    fi
fi
if [[ ! -x "$binary" ]]; then
    echo "missing binary $binary" >&2
    exit 2
fi

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
run_dir="dist/soak/$stamp"
mkdir -p "$run_dir"
log="$run_dir/app.log"
csv="$run_dir/telemetry.csv"
summary="$run_dir/summary.md"
printf 'elapsed_s,rss_kb,cpu_percent,gpu_util_percent,gpu_mem_mb,gpu_temp_c\n' > "$csv"

args=(--run-seconds "$seconds")
if [[ "${LPMT_SOAK_OPEN_OUTPUTS:-1}" == "1" ]]; then
    args+=(--open-outputs)
fi
if [[ -n "${LPMT_SOAK_PROJECT:-}" ]]; then
    args+=(--project "$LPMT_SOAK_PROJECT")
fi

echo "== soak run =="
echo "duration: ${seconds}s, sample interval: ${interval}s, warm-up: ${warmup}s"
echo "binary:   $binary (${features:-default features})"
echo "project:  ${LPMT_SOAK_PROJECT:-demo}"
echo "output:   $run_dir"

RUST_LOG="${RUST_LOG:-info}" "$binary" "${args[@]}" > "$log" 2>&1 &
app_pid=$!
cleanup() {
    if kill -0 "$app_pid" 2>/dev/null; then
        kill "$app_pid" 2>/dev/null || true
        wait "$app_pid" 2>/dev/null || true
    fi
}
trap cleanup EXIT

start_epoch="$(date +%s)"
while kill -0 "$app_pid" 2>/dev/null; do
    state="$(ps -o stat= -p "$app_pid" 2>/dev/null | awk '{print $1}')"
    if [[ -z "$state" || "$state" == Z* ]]; then
        break
    fi
    now="$(date +%s)"
    elapsed=$((now - start_epoch))
    rss_kb="$(ps -o rss= -p "$app_pid" 2>/dev/null | awk '{print $1}')"
    cpu_pct="$(ps -o %cpu= -p "$app_pid" 2>/dev/null | awk '{print $1}')"
    rss_kb="${rss_kb:-0}"
    cpu_pct="${cpu_pct:-0}"

    gpu_util="NA"
    gpu_mem="NA"
    gpu_temp="NA"
    if command -v nvidia-smi >/dev/null 2>&1; then
        gpu_line="$(nvidia-smi --query-gpu=utilization.gpu,memory.used,temperature.gpu --format=csv,noheader,nounits 2>/dev/null | head -n 1 || true)"
        if [[ -n "$gpu_line" ]]; then
            IFS=',' read -r gpu_util gpu_mem gpu_temp <<< "$gpu_line"
            gpu_util="$(echo "$gpu_util" | tr -d ' ')"
            gpu_mem="$(echo "$gpu_mem" | tr -d ' ')"
            gpu_temp="$(echo "$gpu_temp" | tr -d ' ')"
        fi
    fi

    printf '%s,%s,%s,%s,%s,%s\n' \
        "$elapsed" "$rss_kb" "$cpu_pct" "$gpu_util" "$gpu_mem" "$gpu_temp" >> "$csv"
    sleep "$interval"
done

set +e
wait "$app_pid"
app_status=$?
set -e
trap - EXIT

perf_line="$(grep '^performance: ' "$log" | tail -n 1 || true)"
if [[ -z "$perf_line" ]]; then
    echo "soak failed: performance summary missing; see $log" >&2
    exit 1
fi
if [[ "$app_status" -ne 0 ]]; then
    echo "soak failed: app exited with status $app_status; see $log" >&2
    exit 1
fi

avg_fps="$(sed -n 's/.*avg_fps=\([0-9.]*\).*/\1/p' <<< "$perf_line")"
avg_frame_ms="$(sed -n 's/.*avg_frame_ms=\([0-9.]*\).*/\1/p' <<< "$perf_line")"

stats="$(awk -F, -v warmup="$warmup" '
    NR > 1 && $1 + 0 >= warmup {
        steady += 1;
        if (steady == 1) {
            first = $2; last = $2; max = $2; min = $2;
            cpu_sum = $3; cpu_max = $3;
        } else {
            last = $2;
            if ($2 > max) max = $2;
            if ($2 < min) min = $2;
            cpu_sum += $3;
            if ($3 > cpu_max) cpu_max = $3;
        }
        if ($4 != "NA" && $4 != "") {
            gpu_sum += $4; gpu_n += 1;
            if ($4 > gpu_max) gpu_max = $4;
            if ($5 + 0 > gpu_mem_max) gpu_mem_max = $5 + 0;
            if ($6 + 0 > gpu_temp_max) gpu_temp_max = $6 + 0;
        }
    }
    END {
        if (steady == 0) { print "0 0 0 0 0 0 0 0 0 0 0"; exit }
        printf "%.0f %.0f %.0f %.0f %.2f %.2f %.2f %.0f %.0f %.0f %d\n",
            first, last, max, min,
            cpu_sum / steady, cpu_max,
            gpu_n ? gpu_sum / gpu_n : 0,
            gpu_n ? gpu_max : 0,
            gpu_mem_max,
            gpu_temp_max,
            steady;
    }
' "$csv")"
read -r rss_first rss_last rss_max rss_min cpu_avg cpu_max gpu_avg gpu_max gpu_mem_max gpu_temp_max steady_samples <<< "$stats"
rss_growth_mb="$(awk -v first="$rss_first" -v last="$rss_last" 'BEGIN { printf "%.1f", (last - first) / 1024 }')"
rss_max_mb="$(awk -v rss="$rss_max" 'BEGIN { printf "%.1f", rss / 1024 }')"
samples="$(($(wc -l < "$csv") - 1))"

cat > "$summary" <<EOF
# LPMT soak run

- Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
- Requested duration: ${seconds}s
- Binary: \`$binary\`
- Features: ${features:-default}
- Project: ${LPMT_SOAK_PROJECT:-demo}
- Samples: $samples every ${interval}s
- Steady samples after ${warmup}s warm-up: $steady_samples
- App exit: $app_status

## Performance

- Average FPS: ${avg_fps:-unknown}
- Average frame time: ${avg_frame_ms:-unknown} ms
- Raw summary: \`$perf_line\`

## Process

- RSS first: $((rss_first / 1024)) MiB
- RSS last: $((rss_last / 1024)) MiB
- RSS peak: ${rss_max_mb} MiB
- RSS growth first→last: ${rss_growth_mb} MiB
- CPU average: ${cpu_avg}%
- CPU peak: ${cpu_max}%

## GPU

- GPU utilisation average: ${gpu_avg}%
- GPU utilisation peak: ${gpu_max}%
- GPU memory peak: ${gpu_mem_max} MiB
- GPU temperature peak: ${gpu_temp_max} C

## Artifacts

- Log: \`$log\`
- Telemetry: \`$csv\`
EOF

cat "$summary"
echo
echo "soak summary: $summary"

if awk -v growth="$rss_growth_mb" -v limit="$max_rss_growth_mb" \
    'BEGIN { exit !(growth + 0 > limit + 0) }'; then
    echo "soak failed: RSS grew ${rss_growth_mb} MiB, limit is ${max_rss_growth_mb} MiB" >&2
    exit 1
fi
if [[ -n "$avg_fps" ]] && awk -v fps="$avg_fps" -v min="$min_fps" \
    'BEGIN { exit !(fps + 0 < min + 0) }'; then
    echo "soak failed: average fps $avg_fps is below LPMT_SOAK_MIN_FPS=$min_fps" >&2
    exit 1
fi

echo "soak check passed"
