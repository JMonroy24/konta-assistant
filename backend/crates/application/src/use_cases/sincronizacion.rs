use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;

use domain::{detectar_conflicto_importes, Conflicto, DomainError, Operacion};

use crate::ports::{AplicadorSyncPort, ColaSyncPort, ConflictoRepository, EsperaSyncPort};

const MAX_INTENTOS: u32 = 5;
const ESPERA_INICIAL: Duration = Duration::from_millis(100);

#[derive(Debug, Default)]
pub struct ContadoresSync {
    sincronizadas: AtomicU64,
    reintentos: AtomicU64,
    conflictos: AtomicU64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumenContadoresSync {
    pub sincronizadas: u64,
    pub reintentos: u64,
    pub conflictos: u64,
}

impl ContadoresSync {
    pub fn resumen(&self) -> ResumenContadoresSync {
        ResumenContadoresSync {
            sincronizadas: self.sincronizadas.load(Ordering::Relaxed),
            reintentos: self.reintentos.load(Ordering::Relaxed),
            conflictos: self.conflictos.load(Ordering::Relaxed),
        }
    }

    fn incrementar_sincronizadas(&self) {
        self.sincronizadas.fetch_add(1, Ordering::Relaxed);
    }

    fn incrementar_reintentos(&self) {
        self.reintentos.fetch_add(1, Ordering::Relaxed);
    }

    fn incrementar_conflictos(&self) {
        self.conflictos.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct SincronizarCola {
    cola: Arc<dyn ColaSyncPort>,
    aplicador: Arc<dyn AplicadorSyncPort>,
    espera: Arc<dyn EsperaSyncPort>,
    contadores: Arc<ContadoresSync>,
}

impl SincronizarCola {
    pub fn new(
        cola: Arc<dyn ColaSyncPort>,
        aplicador: Arc<dyn AplicadorSyncPort>,
        espera: Arc<dyn EsperaSyncPort>,
        contadores: Arc<ContadoresSync>,
    ) -> Self {
        Self {
            cola,
            aplicador,
            espera,
            contadores,
        }
    }

    pub async fn ejecutar(&self) -> Result<ResumenContadoresSync, DomainError> {
        let pendientes = self.cola.pendientes().await?;
        let mut errores = Vec::new();

        for item in pendientes {
            let mut resultado_final = None;
            for intento in 1..=MAX_INTENTOS {
                if intento > 1 {
                    self.contadores.incrementar_reintentos();
                    self.espera
                        .esperar(ESPERA_INICIAL * 2_u32.pow(intento - 2))
                        .await?;
                }

                let resultado = async {
                    let aplicado = self.aplicador.aplicar_idempotente(&item).await?;
                    self.cola.marcar_aplicada(&item.id).await?;
                    Ok::<_, DomainError>(aplicado)
                }
                .await;

                match resultado {
                    Ok(resultado) => {
                        match resultado {
                            crate::ports::ResultadoAplicacionSync::Aplicada
                            | crate::ports::ResultadoAplicacionSync::YaAplicada => {
                                self.contadores.incrementar_sincronizadas();
                            }
                            crate::ports::ResultadoAplicacionSync::Conflicto => {
                                self.contadores.incrementar_conflictos();
                            }
                        }
                        resultado_final = Some(Ok(()));
                        break;
                    }
                    Err(error) if intento == MAX_INTENTOS => {
                        resultado_final = Some(Err(error.to_string()));
                    }
                    Err(_) => {}
                }
            }

            if let Some(Err(error)) = resultado_final {
                errores.push(format!("{}: {error}", item.id));
            }
        }

        if errores.is_empty() {
            Ok(self.contadores.resumen())
        } else {
            Err(DomainError::ValidacionError(format!(
                "No se pudieron sincronizar {} elemento(s): {}",
                errores.len(),
                errores.join("; ")
            )))
        }
    }
}

pub struct DetectarConflictos {
    conflictos: Arc<dyn ConflictoRepository>,
    contadores: Arc<ContadoresSync>,
}

impl DetectarConflictos {
    pub fn new(conflictos: Arc<dyn ConflictoRepository>, contadores: Arc<ContadoresSync>) -> Self {
        Self {
            conflictos,
            contadores,
        }
    }

    pub async fn ejecutar(
        &self,
        ganadora: &Operacion,
        perdedoras: &[Operacion],
    ) -> Result<Option<Conflicto>, DomainError> {
        let Some(conflicto) = detectar_conflicto_importes(ganadora, perdedoras) else {
            return Ok(None);
        };
        let guardado = self.conflictos.guardar_conflicto(conflicto).await?;
        self.contadores.incrementar_conflictos();
        Ok(Some(guardado))
    }
}

#[cfg(all(test, feature = "testing"))]
#[path = "sincronizacion_tests.rs"]
mod tests;
