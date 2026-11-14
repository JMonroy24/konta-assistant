#!/usr/bin/env python3
"""Mide la propagación de documentos entre dos nodos CouchDB."""

import argparse
import base64
import csv
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path


LOCAL_URL = os.environ.get("COUCHDB_LOCAL_URL", "http://localhost:5985").rstrip("/")
REMOTE_URL = os.environ.get("COUCHDB_REMOTE_URL", "http://localhost:5984").rstrip("/")
DATABASE = os.environ.get("COUCHDB_DATABASE", "bench-replication")
USERNAME = os.environ.get("COUCHDB_USER", "admin")
PASSWORD = os.environ.get("COUCHDB_PASSWORD", "adminsecretpassword")


class CouchDBError(Exception):
    """Error de comunicación o respuesta inesperada de CouchDB."""


def authorization_header():
    if not USERNAME:
        return {}
    encoded = base64.b64encode(f"{USERNAME}:{PASSWORD}".encode("utf-8")).decode("ascii")
    return {"Authorization": f"Basic {encoded}"}


def request_json(url, method="GET", body=None, timeout=10, allowed_statuses=()):
    headers = {"Accept": "application/json", **authorization_header()}
    data = None
    if body is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(body).encode("utf-8")
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            payload = response.read()
            status = response.status
    except urllib.error.HTTPError as error:
        if error.code not in allowed_statuses:
            raise CouchDBError(
                f"CouchDB respondió HTTP {error.code} al atender {method} "
                f"{urllib.parse.urlsplit(url).path}."
            ) from error
        payload = error.read()
        status = error.code
    except (urllib.error.URLError, TimeoutError) as error:
        raise CouchDBError(
            f"No se pudo completar {method} {urllib.parse.urlsplit(url).path}: {error}."
        ) from error

    if status in allowed_statuses and not payload:
        return {}
    try:
        return json.loads(payload.decode("utf-8")) if payload else {}
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise CouchDBError(
            f"CouchDB devolvió una respuesta JSON inválida para "
            f"{method} {urllib.parse.urlsplit(url).path}."
        ) from error


def database_url(base_url):
    return f"{base_url}/{urllib.parse.quote(DATABASE, safe='')}"


def url_with_credentials(base_url):
    parsed = urllib.parse.urlsplit(base_url)
    if not USERNAME:
        return base_url
    netloc = (
        f"{urllib.parse.quote(USERNAME, safe='')}:"
        f"{urllib.parse.quote(PASSWORD, safe='')}@{parsed.netloc}"
    )
    return urllib.parse.urlunsplit(
        (parsed.scheme, netloc, parsed.path, parsed.query, parsed.fragment)
    )


def ensure_database(base_url):
    request_json(
        database_url(base_url),
        method="PUT",
        allowed_statuses=(412,),
    )


def iso_now():
    return datetime.now(timezone.utc).isoformat(timespec="microseconds").replace(
        "+00:00", "Z"
    )


def wait_for_replica(doc_id, since, timeout_seconds):
    deadline = time.monotonic() + timeout_seconds
    sequence = since
    changes_url = f"{database_url(REMOTE_URL)}/_changes"

    while time.monotonic() < deadline:
        remaining = max(1, int((deadline - time.monotonic()) * 1000))
        query = urllib.parse.urlencode(
            {
                "feed": "longpoll",
                "include_docs": "true",
                "since": sequence,
                "timeout": min(remaining, 5000),
            }
        )
        response = request_json(
            f"{changes_url}?{query}",
            timeout=max(10, min(timeout_seconds, 15)),
        )
        sequence = response.get("last_seq", sequence)
        for change in response.get("results", []):
            if change.get("id") == doc_id:
                document = change.get("doc", {})
                if document.get("_id") != doc_id:
                    raise CouchDBError(
                        f"El feed remoto notificó {doc_id}, pero no incluyó el documento."
                    )
                return document, datetime.now(timezone.utc)

    raise CouchDBError(
        f"El documento {doc_id} no apareció en el CouchDB remoto dentro de "
        f"{numero(timeout_seconds)} segundos."
    )


def numero(value):
    return f"{value:g}"


def measure_one(index, timeout_seconds):
    remote_changes = f"{database_url(REMOTE_URL)}/_changes"
    initial = request_json(
        f"{remote_changes}?{urllib.parse.urlencode({'since': 'now', 'limit': 1})}"
    )
    if "last_seq" not in initial:
        raise CouchDBError("CouchDB remoto no devolvió last_seq en _changes.")

    doc_id = f"bench-replication-{uuid.uuid4().hex}"
    origin = iso_now()
    document = {
        "_id": doc_id,
        "type": "benchmark_replication",
        "sequence": index,
        "timestamp_origen": origin,
    }
    request_json(
        database_url(LOCAL_URL) + "/" + urllib.parse.quote(doc_id, safe=""),
        method="PUT",
        body=document,
    )

    replication = {
        "source": url_with_credentials(database_url(LOCAL_URL)),
        "target": url_with_credentials(database_url(REMOTE_URL)),
        "doc_ids": [doc_id],
    }
    result = request_json(
        f"{LOCAL_URL}/_replicate",
        method="POST",
        body=replication,
        timeout=timeout_seconds,
    )
    if result.get("ok") is not True:
        raise CouchDBError("CouchDB no confirmó la replicación solicitada.")

    received, received_time = wait_for_replica(
        doc_id, initial["last_seq"], timeout_seconds
    )
    origin_time = datetime.fromisoformat(
        received["timestamp_origen"].replace("Z", "+00:00")
    )
    return {
        "document_id": doc_id,
        "timestamp_origen": received["timestamp_origen"],
        "timestamp_observado_utc": received_time.isoformat(
            timespec="microseconds"
        ).replace("+00:00", "Z"),
        "latencia_replicacion_ms": (received_time - origin_time).total_seconds() * 1000,
    }


def main():
    parser = argparse.ArgumentParser(
        description="Mide la latencia local-remoto de replicación de CouchDB."
    )
    parser.add_argument("--count", type=int, default=10, help="Cantidad de muestras.")
    parser.add_argument(
        "--timeout",
        type=float,
        default=60,
        help="Tiempo máximo en segundos para encontrar cada documento replicado.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("bench/results")
        / f"replication_latency_{datetime.now(timezone.utc):%Y%m%dT%H%M%SZ}.csv",
        help="Ruta del CSV de salida.",
    )
    args = parser.parse_args()

    if args.count < 1:
        parser.error("--count debe ser al menos 1.")
    if args.timeout <= 0:
        parser.error("--timeout debe ser mayor que cero.")
    if not DATABASE:
        parser.error("COUCHDB_DATABASE no puede estar vacío.")

    try:
        ensure_database(LOCAL_URL)
        ensure_database(REMOTE_URL)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("w", newline="", encoding="utf-8") as file:
            columns = [
                "document_id",
                "timestamp_origen",
                "timestamp_observado_utc",
                "latencia_replicacion_ms",
            ]
            writer = csv.DictWriter(file, fieldnames=columns)
            writer.writeheader()
            for index in range(1, args.count + 1):
                result = measure_one(index, args.timeout)
                result["latencia_replicacion_ms"] = (
                    f"{result['latencia_replicacion_ms']:.3f}"
                )
                writer.writerow(result)
                file.flush()
                print(
                    f"Muestra {index}/{args.count}: "
                    f"{result['latencia_replicacion_ms']} ms"
                )
    except (CouchDBError, OSError, KeyError, ValueError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1

    print(f"CSV guardado en {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
