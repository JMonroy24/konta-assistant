#!/usr/bin/env python3
"""Genera datos SINTETICOS para un negocio (ventas, compras, facturas simuladas, recordatorios,
declaraciones). Solo libreria estandar. Determinista por --seed (reproducible para la Fase 5).
  python3 generate_seed.py demo1 --meses 6 --ops-mes 40 --seed 42 --salida seed_demo1.json
  COUCH_ADMIN_USER=.. COUCH_ADMIN_PASS=.. python3 generate_seed.py demo1 --post http://localhost:5984
Los datos NO son reales: el estudio de cumplimiento debe declararse como sintetico.
"""
import argparse, base64, json, os, random, urllib.request, uuid
from datetime import date, datetime, timedelta, timezone
from decimal import Decimal, ROUND_HALF_UP

Q2 = Decimal("0.01")
IVA = Decimal("0.13")
PAGO_CUENTA = Decimal("0.0175")   # igual que la regla semilla; cambiar si la normativa verificada difiere


def money(d):
    return str(d.quantize(Q2, ROUND_HALF_UP))


def meses_hasta(hasta, n):
    y, m = map(int, hasta.split("-"))
    out = []
    for _ in range(n):
        out.append((y, m))
        m -= 1
        if m == 0:
            y, m = y - 1, 12
    return list(reversed(out))


def decimo_dia_habil(y, m):
    """Decimo dia habil del mes siguiente (lunes a viernes; NO considera feriados)."""
    y, m = (y + 1, 1) if m == 12 else (y, m + 1)
    d, n = date(y, m, 1), 0
    while True:
        if d.weekday() < 5:
            n += 1
            if n == 10:
                return d
        d += timedelta(days=1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("negocio_id")
    ap.add_argument("--meses", type=int, default=6)
    ap.add_argument("--ops-mes", type=int, default=40)
    ap.add_argument("--hasta", default="2026-10", help="ultimo mes con movimiento AAAA-MM")
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--salida", default=None)
    ap.add_argument("--post", default=None, help="URL de CouchDB para cargar con _bulk_docs")
    a = ap.parse_args()

    rng = random.Random(a.seed)
    hoy = date(2026, 11, 8)
    usados, docs, fact_n = set(), [], 0

    def nuevo_id(prefijo, fecha):
        while True:
            u = uuid.UUID(int=rng.getrandbits(128), version=4).hex[:8]
            i = f"{prefijo}:{fecha}:{u}"
            if i not in usados:
                usados.add(i)
                return i

    docs.append({"_id": "negocio:perfil", "tipo": "negocio", "nombre": f"Microempresa sintetica {a.negocio_id}",
                 "tipo_contribuyente": "natural", "nit": "0000-000000-000-0", "sintetico": True})

    for (y, m) in meses_hasta(a.hasta, a.meses):
        periodo = f"{y}-{m:02d}"
        deb = cre = ing = Decimal("0")
        for _ in range(a.ops_mes):
            tipo = "venta" if rng.random() < 0.6 else "compra"
            f = date(y, m, rng.randint(1, 28)).isoformat()
            base = Decimal(rng.randint(500, 150000)) / 100
            iva = (base * IVA).quantize(Q2, ROUND_HALF_UP)
            total = base + iva
            _id = nuevo_id(tipo, f)
            docs.append({"_id": _id, "tipo": tipo, "fecha": f,
                         "descripcion": f"{tipo} sintetica {rng.randint(1, 9999)}",
                         "base": money(base), "iva": money(iva), "total": money(total),
                         "contraparte": {"nombre": f"Contraparte {rng.randint(1, 50)}"},
                         "estado": "registrada", "creado_por": "usuario:sintetico",
                         "origen": "local" if rng.random() < 0.7 else "remoto",
                         "creado_en": f"{f}T12:00:00Z"})
            if tipo == "venta":
                deb += iva; ing += base
                if rng.random() < 0.85:
                    fact_n += 1
                    r = rng.random()
                    est = "aceptado_simulado" if r < 0.70 else ("pendiente" if r < 0.90 else "rechazado")
                    fac = {"_id": f"factura:{f}:{_id.split(':')[2]}", "tipo": "factura",
                           "numero_control": f"SIM-{fact_n:010d}", "venta_id": _id, "estado": est,
                           "hora_emision": f"{f}T{rng.randint(8, 18):02d}:{rng.randint(0, 59):02d}:00Z",
                           "simulado": True}
                    if est == "aceptado_simulado":
                        fac["sello_simulado"] = "SIM-SELLO-" + uuid.UUID(int=rng.getrandbits(128), version=4).hex[:12]
                    docs.append(fac)
            else:
                cre += iva
        neto = deb - cre
        limite = decimo_dia_habil(y, m)
        cerrado = limite < hoy
        for oblig in ("iva_f07", "pago_cuenta_f14"):
            docs.append({"_id": f"recordatorio:{periodo}:{oblig}", "tipo": "recordatorio", "obligacion": oblig,
                         "periodo": periodo, "fecha_limite": limite.isoformat(),
                         "estado": "cumplido" if cerrado else "pendiente"})
        if cerrado:
            docs.append({"_id": f"declaracion:{periodo}:iva_f07", "tipo": "declaracion", "obligacion": "iva_f07",
                         "periodo": periodo, "estado": "presentada",
                         "fecha_presentacion": (limite - timedelta(days=2)).isoformat(),
                         "montos": {"iva_debito": money(deb), "iva_credito": money(cre),
                                    "iva_a_pagar": money(neto) if neto > 0 else "0.00",
                                    "remanente": money(-neto) if neto < 0 else "0.00"}})
            docs.append({"_id": f"declaracion:{periodo}:pago_cuenta_f14", "tipo": "declaracion",
                         "obligacion": "pago_cuenta_f14", "periodo": periodo, "estado": "presentada",
                         "fecha_presentacion": (limite - timedelta(days=1)).isoformat(),
                         "montos": {"ingresos_brutos": money(ing),
                                    "pago_cuenta": money(ing * PAGO_CUENTA)}})
        else:
            docs.append({"_id": f"declaracion:{periodo}:iva_f07", "tipo": "declaracion", "obligacion": "iva_f07",
                         "periodo": periodo, "estado": "borrador",
                         "montos": {"iva_debito": money(deb), "iva_credito": money(cre)}})

    payload = {"docs": docs}
    salida = a.salida or f"seed_{a.negocio_id}.json"
    with open(salida, "w", encoding="utf-8") as fh:
        json.dump(payload, fh, ensure_ascii=False, indent=1)
    print(f"{len(docs)} documentos -> {salida}")

    if a.post:
        user, pw = os.environ["COUCH_ADMIN_USER"], os.environ["COUCH_ADMIN_PASS"]
        tok = base64.b64encode(f"{user}:{pw}".encode()).decode()
        req = urllib.request.Request(f"{a.post}/negocio_{a.negocio_id}/_bulk_docs",
                                     data=json.dumps(payload).encode(), method="POST",
                                     headers={"Content-Type": "application/json", "Authorization": f"Basic {tok}"})
        res = json.load(urllib.request.urlopen(req))
        errores = [r for r in res if "error" in r]
        print(f"cargados: {len(res) - len(errores)}  errores: {len(errores)}")
        for e in errores[:10]:
            print("  ", e)


if __name__ == "__main__":
    main()
