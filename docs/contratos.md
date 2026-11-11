# Contratos de backend

Guía de consulta para Josué, Kevin, William y Benito. Los puertos definen los
límites de integración, los casos de uso coordinan operaciones y `tax-engine`
expone cálculos puros sobre reglas y datos del dominio. `application` no
depende de `tax-engine`: ambos se conectan mediante adaptadores en el borde de
la aplicación.

## Convenciones compartidas

- Errores de casos de uso y adaptadores: `Result<T, DomainError>`.
- Los puertos se inyectan como `Arc<dyn Puerto>`.
- Valores monetarios: `rust_decimal::Decimal`; JSON los representa como cadenas.
- `Periodo` se construye con `Periodo::parse("AAAA-MM")`.
- Los `async fn` abajo son firmas de los traits/casos de uso; los constructores
  síncronos se indican aparte.

## Puertos de `application`

### Persistencia de negocio

```rust
#[async_trait]
trait OperacionRepository {
    async fn guardar_operacion(
        &self, negocio_id: &str, operacion: Operacion
    ) -> Result<Operacion, DomainError>;
    async fn obtener_operacion(
        &self, negocio_id: &str, id: &str
    ) -> Result<Option<Operacion>, DomainError>;
    async fn listar_por_periodo(
        &self, negocio_id: &str, anio_mes: &str
    ) -> Result<Vec<Operacion>, DomainError>;
}

#[async_trait]
trait FacturaRepository {
    async fn guardar_factura(
        &self, negocio_id: &str, factura: Factura
    ) -> Result<Factura, DomainError>;
    async fn obtener_factura(
        &self, negocio_id: &str, id: &str
    ) -> Result<Option<Factura>, DomainError>;
}

#[async_trait]
trait DeclaracionRepository {
    async fn guardar_declaracion(
        &self, negocio_id: &str, declaracion: Declaracion
    ) -> Result<Declaracion, DomainError>;
    async fn obtener_declaracion(
        &self, negocio_id: &str, periodo: &str, tipo_impuesto: TipoImpuesto
    ) -> Result<Option<Declaracion>, DomainError>;
}

#[async_trait]
trait RecordatorioRepository {
    async fn listar_proximos(
        &self, negocio_id: &str
    ) -> Result<Vec<Recordatorio>, DomainError>;
    async fn guardar_recordatorio(
        &self, recordatorio: Recordatorio
    ) -> Result<Recordatorio, DomainError>;
}

#[async_trait]
trait ReglaFiscalRepository {
    async fn obtener_regla(
        &self, codigo: &str, fecha: NaiveDate
    ) -> Result<Option<ReglaFiscal>, DomainError>;
    async fn guardar_regla(
        &self, regla: ReglaFiscal
    ) -> Result<ReglaFiscal, DomainError>;
}
```

### Servicios y sincronización

```rust
#[async_trait]
trait HaciendaSimulatorPort {
    async fn validar_documento(
        &self, factura: &Factura
    ) -> Result<Result<String, String>, DomainError>;
}

#[async_trait]
trait CalculadoraFiscalPort {
    async fn tasa_iva_vigente(
        &self, reglas: &[ReglaFiscal], fecha: NaiveDate
    ) -> Result<Decimal, DomainError>;
    async fn calcular_resumen(
        &self,
        periodo: Periodo,
        operaciones: &[Operacion],
        reglas: &[ReglaFiscal],
        remanente_periodo_anterior: Option<Decimal>,
    ) -> Result<ResumenImpuestos, DomainError>;
    async fn generar_obligaciones(
        &self, periodo: Periodo, reglas: &[ReglaFiscal], feriados: &[NaiveDate]
    ) -> Result<Vec<ObligacionFiscal>, DomainError>;
}

#[async_trait]
trait ConflictoRepository {
    async fn guardar_conflicto(
        &self, conflicto: Conflicto
    ) -> Result<Conflicto, DomainError>;
    async fn listar_pendientes(&self) -> Result<Vec<Conflicto>, DomainError>;
}

trait RelojPort {
    fn ahora_utc(&self) -> DateTime<Utc>;
    fn hoy(&self) -> NaiveDate;
}

#[async_trait]
trait NotificacionPort {
    async fn notificar_recordatorio(
        &self, recordatorio: &Recordatorio
    ) -> Result<(), DomainError>;
}

#[async_trait]
trait SyncStatusPort {
    async fn obtener_estado(
        &self, negocio_id: &str
    ) -> Result<serde_json::Value, DomainError>;
}
```

`ItemSync` transporta `id: String`, `endpoint: String`,
`payload: serde_json::Value` y `timestamp: DateTime<Utc>`.

```rust
#[async_trait]
trait ColaSyncPort {
    async fn encolar(&self, item: ItemSync) -> Result<(), DomainError>;
    async fn pendientes(&self) -> Result<Vec<ItemSync>, DomainError>;
    async fn marcar_aplicada(&self, id_cliente: &str) -> Result<(), DomainError>;
}

enum ResultadoAplicacionSync { Aplicada, YaAplicada, Conflicto }

#[async_trait]
trait AplicadorSyncPort {
    async fn aplicar_idempotente(
        &self, item: &ItemSync
    ) -> Result<ResultadoAplicacionSync, DomainError>;
}

#[async_trait]
trait EsperaSyncPort {
    async fn esperar(&self, duracion: Duration) -> Result<(), DomainError>;
}
```

`AplicadorSyncPort::aplicar_idempotente` debe aplicar de forma atómica usando
`item.id` como clave: una repetición devuelve `YaAplicada` y no crea un segundo
efecto de negocio.

## Casos de uso

Los constructores reciben los puertos indicados como `Arc<dyn ...>`.

| Caso de uso | Constructor | Ejecución |
|---|---|---|
| `RegistrarOperacion` | `new(Arc<dyn OperacionRepository>, Arc<dyn ReglaFiscalRepository>, Arc<dyn CalculadoraFiscalPort>)` | `ejecutar(&self, negocio_id: &str, operacion: Operacion) -> Result<Operacion, DomainError>` |
| `ConsultarResumenImpuestos` | `new(Arc<dyn OperacionRepository>, Arc<dyn ReglaFiscalRepository>, Arc<dyn CalculadoraFiscalPort>)` | `ejecutar(&self, negocio_id: &str, periodo: Periodo, remanente_periodo_anterior: Option<Decimal>) -> Result<ResumenImpuestos, DomainError>` |
| `GenerarBorradorDeclaracion` | `new(Arc<dyn OperacionRepository>, Arc<dyn ReglaFiscalRepository>, Arc<dyn DeclaracionRepository>, Arc<dyn CalculadoraFiscalPort>)` | `ejecutar(&self, negocio_id: &str, periodo: Periodo, tipo_impuesto: TipoImpuesto) -> Result<Declaracion, DomainError>` |
| `ListarRecordatorios` | `new(Arc<dyn ReglaFiscalRepository>, Arc<dyn RecordatorioRepository>, Arc<dyn CalculadoraFiscalPort>)` | `ejecutar(&self, negocio_id: &str, periodo: Periodo, feriados: &[NaiveDate]) -> Result<Vec<Recordatorio>, DomainError>` |
| `EmitirFactura` | `new(Arc<dyn FacturaRepository>, Arc<dyn HaciendaSimulatorPort>, Arc<dyn RelojPort>)` | `ejecutar(&self, negocio_id: &str, referencia_venta_id: &str, numero_control_simulado: &str) -> Result<Factura, DomainError>` |
| `SincronizarCola` | `new(Arc<dyn ColaSyncPort>, Arc<dyn AplicadorSyncPort>, Arc<dyn EsperaSyncPort>, Arc<ContadoresSync>)` | `ejecutar(&self) -> Result<ResumenContadoresSync, DomainError>` |
| `DetectarConflictos` | `new(Arc<dyn ConflictoRepository>, Arc<ContadoresSync>)` | `ejecutar(&self, ganadora: &Operacion, perdedoras: &[Operacion]) -> Result<Option<Conflicto>, DomainError>` |

`SincronizarCola` incrementa los contadores atómicos de sincronizadas,
reintentos y conflictos. Ante errores de aplicación reintenta hasta cinco
intentos totales con esperas exponenciales (100, 200, 400 y 800 ms); un elemento
que agota intentos permanece pendiente. `DetectarConflictos` solo describe y
guarda la diferencia; no modifica ni fusiona operaciones.

### Ejemplo de inyección de un caso de uso

El ensamblador de infraestructura proporciona implementaciones concretas que
cumplen los traits; el caso de uso solo conoce los puertos:

```rust,ignore
let registrar = RegistrarOperacion::new(
    operaciones_repo, // Arc<dyn OperacionRepository>
    reglas_repo,      // Arc<dyn ReglaFiscalRepository>
    calculadora,      // Arc<dyn CalculadoraFiscalPort>
);
let guardada = registrar
    .ejecutar("negocio_001", operacion)
    .await?;
```

Ejemplo de uso para el conteo que puede exponer la telemetría:

```rust,ignore
let contadores = Arc::new(ContadoresSync::default());
let sync = SincronizarCola::new(cola, aplicador, espera, contadores.clone());
sync.ejecutar().await?;
let metricas = contadores.resumen();
println!(
    "sincronizadas={}, reintentos={}, conflictos={}",
    metricas.sincronizadas, metricas.reintentos, metricas.conflictos
);
```

## Funciones públicas de `tax-engine`

Tipos de apoyo: `ParametrosFiscales { tasa_iva: Decimal,
tasa_pago_a_cuenta: Decimal }`, `calendario::Obligacion { tipo:
TipoObligacion, fecha_limite: NaiveDate }`, `renta::ResultadoRenta {
impuesto_causado: Decimal, pagos_a_cuenta: Decimal, saldo: Decimal,
explicacion: String }`, `renta::Tramo { desde: Decimal, hasta: Option<Decimal>,
tasa: Decimal, cuota_fija: Decimal, exceso_sobre: Decimal }` y
`multas::MultaEstimada { monto_multa: Decimal, dias_atraso: u32 }`.

### Motor de IVA y selección de reglas

```rust
ParametrosFiscales::desde_reglas(
    reglas: &[ReglaFiscal],
    fecha: NaiveDate,
) -> Result<ParametrosFiscales, DomainError>

MotorFiscal::calcular_impuestos(
    periodo: Periodo,
    operaciones: &[Operacion],
    params: &ParametrosFiscales,
    remanente_periodo_anterior: Option<Decimal>,
) -> Result<ResumenImpuestos, DomainError>

reglas::seleccionar_regla<'a>(
    reglas: &'a [ReglaFiscal],
    codigo: &str,
    fecha: NaiveDate,
) -> Result<&'a ReglaFiscal, DomainError>
```

El selector requiere vigencia para la fecha, elige la versión mayor cuando hay
más de una candidata y descarta reglas marcadas `por_definir`.

### Calendario fiscal

```rust
calendario::fecha_limite(
    regla: &ReglaFiscal,
    periodo: &Periodo,
    feriados: &[NaiveDate],
) -> Result<NaiveDate, DomainError>

calendario::generar_obligaciones(
    periodo: &Periodo,
    reglas: &[ReglaFiscal],
    feriados: &[NaiveDate],
) -> Vec<calendario::Obligacion>
```

Modos públicos de plazo: `MODO_DIAS_HABILES_MES_SIGUIENTE`,
`MODO_FECHA_FIJA_ANUAL`, `CODIGO_PLAZO_IVA_F07`,
`CODIGO_PLAZO_PAGO_CUENTA_F14` y `CODIGO_PLAZO_RENTA_ANUAL_F11`.

### Impuesto sobre la renta

```rust
renta::renta_persona_natural(
    renta_neta: Decimal,
    regla: &ReglaFiscal,
    pagos_a_cuenta: Decimal,
    fecha_limite: NaiveDate,
) -> Result<renta::ResultadoRenta, DomainError>

renta::renta_persona_juridica(
    ingresos_anuales: Decimal,
    renta_neta: Decimal,
    regla: &ReglaFiscal,
    pagos_a_cuenta: Decimal,
    fecha_limite: NaiveDate,
) -> Result<renta::ResultadoRenta, DomainError>
```

Códigos de reglas: `CODIGO_RENTA_NATURAL_TRAMOS` y
`CODIGO_RENTA_JURIDICA`.

### Multas estimadas

```rust
multas::estimar_multa(
    regla: &ReglaFiscal,
    monto: Decimal,
    dias_atraso: u32,
) -> Result<multas::MultaEstimada, DomainError>
```

Solo calcula cuando la regla está verificada y tiene porcentaje diario; no
completa valores normativos faltantes.

### Explicaciones

```rust
explicacion::explicacion_iva(
    resumen: &ResumenImpuestos, fecha_limite: NaiveDate
) -> String
explicacion::explicacion_pago_a_cuenta(
    resumen: &ResumenImpuestos, fecha_limite: NaiveDate
) -> String
explicacion::explicacion_renta_natural(
    renta_neta: Decimal, tramo: Option<&renta::Tramo>, impuesto_causado: Decimal,
    pagos_a_cuenta: Decimal, saldo: Decimal, fecha_limite: NaiveDate
) -> String
explicacion::explicacion_renta_juridica(
    ingresos_anuales: Decimal, renta_neta: Decimal, umbral: Decimal,
    tasa: Decimal, es_reducida: bool, impuesto_causado: Decimal,
    pagos_a_cuenta: Decimal, saldo: Decimal, fecha_limite: NaiveDate
) -> String
explicacion::explicacion_multa(
    monto: Decimal, dias_atraso: u32, fecha_limite: NaiveDate
) -> String
```

### Ejemplo de cálculo fiscal

```rust,ignore
let parametros = ParametrosFiscales::desde_reglas(&reglas, fecha)?;
let resumen = MotorFiscal::calcular_impuestos(
    Periodo::parse("2026-11")?,
    &operaciones,
    &parametros,
    None,
)?;
```

## Pruebas por módulo

Recuento de pruebas unitarias listadas por `cargo test -- --list`; es cobertura
de escenarios de prueba por módulo, **no** porcentaje de líneas o ramas
ejecutadas. `cargo-llvm-cov` no está instalado en este entorno.

| Crate | Módulo | Pruebas |
|---|---|---:|
| `domain` | `entities` | 12 |
| `domain` | `value_objects` | 14 |
| `application` (`testing`) | `testing` | 2 |
| `application` (`testing`) | `use_cases` | 6 |
| `application` (`testing`) | `use_cases::sincronizacion` | 5 |
| `tax-engine` | `calendario` | 8 |
| `tax-engine` | `explicacion` | 4 |
| `tax-engine` | `multas` | 5 |
| `tax-engine` | `reglas` | 6 |
| `tax-engine` | `renta` | 8 |
| `tax-engine` | raíz (`MotorFiscal`) | 8 |
| **Total** | | **78** |

La suite solicitada sin feature de pruebas reporta 26 pruebas de `domain`, 39
de `tax-engine` y 0 de `application`, porque sus pruebas de casos de uso están
condicionadas por `feature = "testing"`. Habilitándola, `application` aporta
13 pruebas.
