use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Importe inválido: el valor debe ser positivo y numérico")]
    ImporteInvalido,

    #[error("Formato de fecha inválido: {0}")]
    FechaInvalida(String),

    #[error("Regla fiscal no encontrada o fuera de vigencia para el período: {0}")]
    ReglaFiscalNoEncontrada(String),

    #[error("Conflicto de concurrencia en documento: {0}")]
    ConflictoRevision(String),

    #[error("Entidad no encontrada: {0}")]
    NotFound(String),

    #[error("Error de validación: {0}")]
    ValidacionError(String),
}
