import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 20 },  // Ramp-up
    { duration: '1m', target: 50 },   // Carga sostenida
    { duration: '30s', target: 0 },   // Ramp-down
  ],
  thresholds: {
    http_req_duration: ['p(95)<300', 'p(99)<500'], // Latencia p95 y p99
    http_req_failed: ['rate<0.01'],                 // Tasa de fallos < 1%
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function () {
  // 1. Salud
  const resSalud = http.get(`${BASE_URL}/salud`);
  check(resSalud, {
    'salud status es 200': (r) => r.status === 200,
  });

  // 2. Consulta de operaciones (Mango Query)
  const resOps = http.get(`${BASE_URL}/operaciones?mes=2026-11`);
  check(resOps, {
    'operaciones status es 200': (r) => r.status === 200,
  });

  // 3. Resumen de impuestos
  const resImp = http.get(`${BASE_URL}/impuestos/resumen?mes=2026-11`);
  check(resImp, {
    'resumen status es 200': (r) => r.status === 200,
  });

  sleep(0.5);
}
