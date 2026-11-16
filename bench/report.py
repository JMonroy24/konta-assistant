#!/usr/bin/env python3
"""Genera un resumen legible de los resultados de k6 y docker stats."""

import argparse
import csv
import json
import math
import re
import sys
from collections import defaultdict
from pathlib import Path


def numero(valor, decimales=2):
    """Formatea números con separadores habituales en español."""
    formatted = f"{valor:,.{decimales}f}"
    return formatted.replace(",", "\0").replace(".", ",").replace("\0", ".")


def entero(valor):
    return numero(valor, 0)


def cargar_json(path):
    try:
        with path.open(encoding="utf-8") as file:
            data = json.load(file)
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"No se pudo leer {path}: {error}") from error
    if not isinstance(data, dict) or not isinstance(data.get("metrics"), dict):
        raise ValueError(f"El archivo no tiene el formato de resumen k6 esperado: {path}")
    return data


def valor_metrica(metrics, name, key, fallback="—"):
    metric = metrics.get(name, {})
    value = metric.get("values", metric).get(key)
    return fallback if value is None else value


def resumen_k6(path):
    data = cargar_json(path)
    metrics = data["metrics"]
    attempted = valor_metrica(metrics, "documents_attempted", "count")
    processed = valor_metrica(metrics, "documents_processed", "count")
    throughput = valor_metrica(metrics, "documents_processed", "rate")
    latency = metrics.get("document_latency", {})
    checks = metrics.get("checks", {})
    check_values = checks.get("values", checks)
    passed = check_values.get("passes", 0)
    failed = check_values.get("fails", 0)
    total_checks = passed + failed
    check_result = (
        f"{entero(passed)} / {entero(total_checks)}"
        + (f" ({numero(passed / total_checks * 100)}%)" if total_checks else "")
    )

    return {
        "name": path.stem.removesuffix("-summary"),
        "scenario": data.get("test_scenario", "—"),
        "resilience": data.get("resilience", {}).get("success_percent"),
        "attempted": attempted,
        "processed": processed,
        "throughput": throughput,
        "p50": latency.get("p(50)"),
        "p95": latency.get("p(95)"),
        "p99": latency.get("p(99)"),
        "checks": check_result,
        "metrics": metrics,
    }


def render_k6(results):
    lines = [
        "## Pruebas de carga",
        "",
        "| Prueba | Escenario | Intentados | Procesados | Documentos/s | Latencia p50 | p95 | p99 | Checks 2xx | Resiliencia |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for result in results:
        latency = [
            "—" if result[key] is None else f"{numero(result[key])} ms"
            for key in ("p50", "p95", "p99")
        ]
        lines.append(
            f"| {result['name']} | {result['scenario']} | {entero(result['attempted'])} "
            f"| {entero(result['processed'])} | {numero(result['throughput'])} "
            f"| {latency[0]} | {latency[1]} | {latency[2]} | {result['checks']} "
            f"| {numero(result['resilience']) + '%' if result['resilience'] is not None else '—'} |"
        )

    threshold_lines = []
    inconsistencies = []
    threshold_pattern = re.compile(r"^(p\(\d+\)|rate)<(\d+(?:\.\d+)?)$")
    for result in results:
        for metric_name, metric in result["metrics"].items():
            thresholds = metric.get("thresholds", {})
            for expression, passed in thresholds.items():
                status = "Cumplido" if passed else "No cumplido"
                threshold_lines.append(
                    f"| {result['name']} | `{metric_name}` | `{expression}` | {status} |"
                )
                match = threshold_pattern.fullmatch(expression)
                if match:
                    stat, limit = match.groups()
                    values = metric.get("values", metric)
                    observed = values.get("value") if stat == "rate" else values.get(stat)
                    if observed is not None and bool(passed) != (observed < float(limit)):
                        inconsistencies.append(
                            f"- **{result['name']} / {metric_name}:** k6 marca "
                            f"`{expression}` como {'cumplido' if passed else 'no cumplido'}, "
                            f"pero el valor resumido es {numero(observed)}."
                        )

    lines.extend(["", "## Umbrales", ""])
    if threshold_lines:
        lines.extend(
            [
                "| Prueba | Métrica | Umbral | Resultado reportado por k6 |",
                "|---|---|---|---|",
                *threshold_lines,
            ]
        )
    else:
        lines.append("Los archivos no incluyen resultados de umbrales.")

    if inconsistencies:
        lines.extend(
            [
                "",
                "> **Revisar:** hay discrepancias entre el estado de umbrales guardado "
                "por k6 y los valores del resumen. Conviene verificar unidades y "
                "configuración antes de interpretar el resultado.",
                "",
                *inconsistencies,
            ]
        )
    lines.append("")
    return lines


def resumen_contenedores(path):
    required = {"timestamp_utc", "container", "cpu_percent"}
    stats = defaultdict(
        lambda: {
            "samples": 0,
            "cpu_total": 0.0,
            "cpu_max": 0.0,
            "memory_max": 0.0,
        }
    )
    timestamps = []
    memory_column = None
    try:
        with path.open(newline="", encoding="utf-8") as file:
            reader = csv.DictReader(file)
            if not reader.fieldnames or not required.issubset(reader.fieldnames):
                raise ValueError(f"El CSV no contiene las columnas esperadas: {path}")
            if "rss_mib" in reader.fieldnames:
                memory_column = "rss_mib"
            elif "memory_used_mib" in reader.fieldnames:
                memory_column = "memory_used_mib"
            else:
                raise ValueError(f"El CSV no contiene una columna RSS o memoria: {path}")
            for row in reader:
                try:
                    cpu = float(row["cpu_percent"])
                    memory = float(row[memory_column])
                except (TypeError, ValueError) as error:
                    raise ValueError(f"Fila inválida en {path}: {row}") from error
                entry = stats[row["container"]]
                entry["samples"] += 1
                entry["cpu_total"] += cpu
                entry["cpu_max"] = max(entry["cpu_max"], cpu)
                entry["memory_max"] = max(entry["memory_max"], memory)
                timestamps.append(row["timestamp_utc"])
    except OSError as error:
        raise ValueError(f"No se pudo leer {path}: {error}") from error

    return stats, timestamps


def render_contenedores(paths):
    lines = ["## Uso de recursos", ""]
    if not paths:
        lines.extend(["No se encontraron archivos de métricas de contenedores.", ""])
        return lines

    for path in paths:
        stats, timestamps = resumen_contenedores(path)
        try:
            with path.open(newline="", encoding="utf-8") as file:
                memory_column = csv.DictReader(file).fieldnames
        except OSError as error:
            raise ValueError(f"No se pudo leer {path}: {error}") from error
        is_rss = "rss_mib" in memory_column
        memory_label = "RSS pico" if is_rss else "Memoria Docker pico (no RSS)"
        lines.extend(
            [
                f"### `{path.name}`",
                "",
                f"Muestras: {entero(len(timestamps))}"
                + (
                    f" · período UTC: {timestamps[0]} a {timestamps[-1]}"
                    if timestamps
                    else ""
                ),
                "",
                f"| Contenedor | CPU promedio | CPU máximo | {memory_label} (MiB) |",
                "|---|---:|---:|---:|",
            ]
        )
        for name, entry in sorted(
            stats.items(), key=lambda item: item[1]["memory_max"], reverse=True
        ):
            average_cpu = entry["cpu_total"] / entry["samples"]
            lines.append(
                f"| {name} | {numero(average_cpu)}% | {numero(entry['cpu_max'])}% "
                f"| {numero(entry['memory_max'])} |"
            )
        lines.append("")
    return lines


def render_replicacion(paths):
    lines = ["## Latencia de replicación CouchDB", ""]
    if not paths:
        lines.extend(["Sin CSV de latencia de replicación.", ""])
        return lines

    lines.extend(
        [
            "| Archivo | Muestras | Promedio (ms) | p95 (ms) | Máximo (ms) |",
            "|---|---:|---:|---:|---:|",
        ]
    )
    for path in paths:
        try:
            with path.open(newline="", encoding="utf-8") as file:
                reader = csv.DictReader(file)
                if not reader.fieldnames or "latencia_replicacion_ms" not in reader.fieldnames:
                    raise ValueError(f"El CSV no contiene latencia de replicación: {path}")
                values = []
                for row in reader:
                    try:
                        values.append(float(row["latencia_replicacion_ms"]))
                    except (TypeError, ValueError) as error:
                        raise ValueError(f"Fila inválida en {path}: {row}") from error
        except OSError as error:
            raise ValueError(f"No se pudo leer {path}: {error}") from error
        if not values:
            raise ValueError(f"No hay muestras de latencia en {path}.")
        ordered = sorted(values)
        p95 = ordered[max(0, math.ceil(len(ordered) * 0.95) - 1)]
        lines.append(
            f"| `{path.name}` | {entero(len(values))} | {numero(sum(values) / len(values))} "
            f"| {numero(p95)} | {numero(max(values))} |"
        )
    lines.append("")
    return lines


def listar_csv(results_dir):
    containers = []
    replication = []
    for path in sorted(results_dir.glob("*.csv")):
        try:
            with path.open(newline="", encoding="utf-8") as file:
                fields = next(csv.reader(file), [])
        except OSError as error:
            raise ValueError(f"No se pudo leer {path}: {error}") from error
        if {"timestamp_utc", "container", "cpu_percent"}.issubset(fields):
            containers.append(path)
        elif "latencia_replicacion_ms" in fields:
            replication.append(path)
    return containers, replication


def main():
    default_results = Path(__file__).resolve().parent / "results"
    parser = argparse.ArgumentParser(
        description="Resume los JSON de k6 y los CSV de docker stats en Markdown."
    )
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=default_results,
        help="Directorio con los archivos de resultados (por defecto: bench/results).",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=default_results / "readable-summary.md",
        help="Archivo Markdown de salida.",
    )
    args = parser.parse_args()

    summaries = sorted(args.results_dir.glob("*-summary.json"))
    if not summaries:
        parser.error(f"No se encontraron archivos *-summary.json en {args.results_dir}")

    try:
        results = [resumen_k6(path) for path in summaries]
        container_files, replication_files = listar_csv(args.results_dir)
        report = [
            "# Resumen de resultados",
            "",
            "Latencias expresadas en milisegundos. Las tasas son documentos por segundo.",
            "",
            *render_k6(results),
            *render_contenedores(container_files),
            *render_replicacion(replication_files),
        ]
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text("\n".join(report), encoding="utf-8")
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1

    print(f"Resumen guardado en {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
