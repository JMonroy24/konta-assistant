#!/usr/bin/env bash
set -euo pipefail

interval_seconds="${INTERVAL_SECONDS:-1}"
if [[ ! "$interval_seconds" =~ ^[1-9][0-9]*$ ]]; then
  printf 'INTERVAL_SECONDS debe ser un entero positivo.\n' >&2
  exit 2
fi

once=false
if [[ "${1:-}" == "--once" ]]; then
  once=true
  shift
fi

default_output="bench/results/container_stats_$(date -u +%Y%m%dT%H%M%SZ).csv"
output_file="${1:-$default_output}"
output_dir="$(dirname -- "$output_file")"
mkdir -p "$output_dir"

if ! command -v docker >/dev/null 2>&1; then
  printf 'No se encontró el comando docker en PATH.\n' >&2
  exit 127
fi

if [[ ! -r /sys/fs/cgroup/cgroup.controllers ]]; then
  printf 'Se requiere cgroups v2 montado en /sys/fs/cgroup para medir RSS.\n' >&2
  exit 1
fi

container_rss_mib() {
  local host_pid="$1"
  local relative_cgroup cgroup_dir pid_file process_pid rss_kib=0 process_rss
  local -A seen_pids=()

  if [[ ! -r "/proc/${host_pid}/cgroup" ]]; then
    printf 'No se puede leer el cgroup del proceso %s.\n' "$host_pid" >&2
    return 1
  fi
  relative_cgroup="$(awk -F: '$1 == "0" { print $3; exit }' "/proc/${host_pid}/cgroup")"
  if [[ -z "$relative_cgroup" || "$relative_cgroup" != /* ]]; then
    printf 'No se encontró una ruta de cgroup v2 para PID %s.\n' "$host_pid" >&2
    return 1
  fi
  cgroup_dir="/sys/fs/cgroup${relative_cgroup}"
  if [[ ! -d "$cgroup_dir" ]]; then
    printf 'No existe el cgroup del contenedor: %s\n' "$cgroup_dir" >&2
    return 1
  fi

  while IFS= read -r -d '' pid_file; do
    while IFS= read -r process_pid; do
      [[ -n "$process_pid" && -z "${seen_pids[$process_pid]+present}" ]] || continue
      seen_pids["$process_pid"]=1
      if [[ ! -r "/proc/${process_pid}/status" ]]; then
        continue
      fi
      process_rss="$(awk '$1 == "VmRSS:" { print $2; found=1 } END { if (!found) exit 1 }' \
        "/proc/${process_pid}/status")" || continue
      [[ "$process_rss" =~ ^[0-9]+$ ]] || continue
      rss_kib=$((rss_kib + process_rss))
    done < "$pid_file"
  done < <(find "$cgroup_dir" -type f -name cgroup.procs -print0)

  if (( ${#seen_pids[@]} == 0 )); then
    printf 'No se encontraron procesos en el cgroup del contenedor %s.\n' "$host_pid" >&2
    return 1
  fi
  awk -v kib="$rss_kib" 'BEGIN { printf "%.3f", kib / 1024 }'
}

printf 'timestamp_utc,container,cpu_percent,rss_mib\n' > "$output_file"
printf 'Guardando métricas en %s (Ctrl+C para detener).\n' "$output_file" >&2

while true; do
  timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  stats="$(docker stats --no-stream --format '{{.Name}},{{.CPUPerc}}')" || {
    printf 'No se pudieron leer las estadísticas de Docker.\n' >&2
    exit 1
  }
  while IFS=, read -r container cpu; do
    [[ -n "$container" ]] || continue
    cpu="${cpu%\%}"
    host_pid="$(docker inspect --format '{{.State.Pid}}' "$container")" || {
      printf 'No se pudo inspeccionar el contenedor %s.\n' "$container" >&2
      exit 1
    }
    if [[ ! "$host_pid" =~ ^[1-9][0-9]*$ ]]; then
      printf 'El contenedor %s no tiene un PID de host válido.\n' "$container" >&2
      exit 1
    fi
    rss_mib="$(container_rss_mib "$host_pid")" || exit 1
    printf '%s,%s,%s,%s\n' "$timestamp" "$container" "$cpu" "$rss_mib" >> "$output_file"
  done <<< "$stats"

  if [[ "$once" == true ]]; then
    break
  fi
  sleep "$interval_seconds"
done
