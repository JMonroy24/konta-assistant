import http from 'k6/http';
import { check } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

const documentsAttempted = new Counter('documents_attempted');
const documentsProcessed = new Counter('documents_processed');
const resilienceRate = new Rate('resilience_rate');
const documentLatency = new Trend('document_latency', true);

const BASE_URL = (__ENV.API_URL || 'http://localhost:8080').replace(/\/$/, '');
const ITERATIONS_PER_TYPE = Number(__ENV.ITERATIONS_PER_TYPE || 1000);
const VUS_PER_TYPE = Number(__ENV.VUS_PER_TYPE || 1);
const TEST_SCENARIO = __ENV.TEST_SCENARIO || 'baseline';
const SUMMARY_JSON = __ENV.SUMMARY_JSON || 'bench/results/summary.json';
const TIMEOUT = __ENV.REQUEST_TIMEOUT || '30s';

if (!Number.isSafeInteger(ITERATIONS_PER_TYPE) || ITERATIONS_PER_TYPE < 1) {
  throw new Error('ITERATIONS_PER_TYPE debe ser un entero positivo.');
}
if (!Number.isSafeInteger(VUS_PER_TYPE) || VUS_PER_TYPE < 1) {
  throw new Error('VUS_PER_TYPE debe ser un entero positivo.');
}

const iterationsPerWorkload = ITERATIONS_PER_TYPE;
const runId = Date.now().toString(36);

export const options = {
  discardResponseBodies: true,
  scenarios: {
    venta: {
      executor: 'per-vu-iterations',
      exec: 'venta',
      vus: VUS_PER_TYPE,
      iterations: iterationsPerWorkload,
      maxDuration: '30m',
    },
    iva_retenido: {
      executor: 'per-vu-iterations',
      exec: 'iva_retenido',
      vus: VUS_PER_TYPE,
      iterations: iterationsPerWorkload,
      maxDuration: '30m',
    },
    factura_electronica: {
      executor: 'per-vu-iterations',
      exec: 'factura_electronica',
      vus: VUS_PER_TYPE,
      iterations: iterationsPerWorkload,
      maxDuration: '30m',
    },
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(50)', 'p(95)', 'p(99)'],
  thresholds: {
    document_latency: ['p(50)<250', 'p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.01'],
  },
};

function makeDocument(type, id, issuedAt) {
  const common = {
    id,
    fecha_emision: issuedAt,
    moneda: 'USD',
  };

  if (type === 'venta') {
    return {
      endpoint: '/ventas',
      payload: {
        ...common,
        tipo: 'venta',
        descripcion: 'Venta generada por prueba de carga',
        base_imponible: '100.00',
        iva: '13.00',
        total: '113.00',
      },
    };
  }

  if (type === 'iva_retenido') {
    return {
      endpoint: '/compras',
      payload: {
        ...common,
        tipo: 'compra',
        descripcion: 'Compra con IVA retenido generada por prueba de carga',
        base_imponible: '100.00',
        iva: '13.00',
        iva_retenido: '13.00',
        total: '100.00',
      },
    };
  }

  return {
    endpoint: '/facturas',
    payload: {
      ...common,
      tipo: 'factura_electronica',
      tipo_dte: '01',
      estado: 'simulada',
      detalle: [{
        descripcion: 'Factura generada por prueba de carga',
        cantidad: '1',
        precio_unitario: '100.00',
        iva: '13.00',
        total: '113.00',
      }],
      resumen: {
        base_imponible: '100.00',
        iva: '13.00',
        total: '113.00',
      },
    },
  };
}

function submitDocument(type) {
  const id = `k6-${runId}-${type}-${__VU}-${__ITER}`;
  const document = makeDocument(type, id, new Date().toISOString());
  const startedAt = Date.now();

  documentsAttempted.add(1, { document_type: type });
  const response = http.post(
    `${BASE_URL}${document.endpoint}`,
    JSON.stringify(document.payload),
    {
      headers: { 'Content-Type': 'application/json' },
      tags: { document_type: type, endpoint: document.endpoint },
      timeout: TIMEOUT,
    },
  );
  documentLatency.add(Date.now() - startedAt, { document_type: type });

  const succeeded = check(response, {
    [`${type} devuelve una respuesta 2xx`]: (res) =>
      res.status >= 200 && res.status < 300,
  });

  resilienceRate.add(succeeded);
  if (succeeded) {
    documentsProcessed.add(1, { document_type: type });
  }
}

export function venta() {
  submitDocument('venta');
}

export function iva_retenido() {
  submitDocument('iva_retenido');
}

export function factura_electronica() {
  submitDocument('factura_electronica');
}

export function handleSummary(data) {
  const rate = data.metrics.resilience_rate.values.rate;
  const attempted = data.metrics.documents_attempted.values.count;
  const processed = data.metrics.documents_processed.values.count;
  const summary = {
    ...data,
    test_scenario: TEST_SCENARIO,
    workload: {
      iterations_per_type: ITERATIONS_PER_TYPE,
      vus_per_type: VUS_PER_TYPE,
      planned_requests_per_type: iterationsPerWorkload * VUS_PER_TYPE,
      planned_total_requests: iterationsPerWorkload * VUS_PER_TYPE * 3,
    },
    resilience: {
      successful_requests: processed,
      failed_or_timed_out_requests: attempted - processed,
      success_rate: rate,
      success_percent: Number((rate * 100).toFixed(2)),
    },
  };
  const serialized = `${JSON.stringify(summary, null, 2)}\n`;
  const consoleSummary = `${JSON.stringify({
    test_scenario: summary.test_scenario,
    workload: summary.workload,
    resilience: summary.resilience,
  }, null, 2)}\n`;

  return {
    stdout: consoleSummary,
    [SUMMARY_JSON]: serialized,
  };
}
