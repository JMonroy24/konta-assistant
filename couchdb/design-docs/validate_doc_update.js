function(newDoc, oldDoc, userCtx, secObj) {
    // Validar tipo requerido
    if (!newDoc.tipo) {
        throw({ forbidden: "El campo 'tipo' es obligatorio en todos los documentos." });
    }

    // Validar documentos de tipo venta o compra
    if (newDoc.tipo === "venta" || newDoc.tipo === "compra") {
        if (!newDoc.fecha || !newDoc.base || !newDoc.iva || !newDoc.total) {
            throw({ forbidden: "Operaciones requieren fecha, base, iva y total." });
        }
        // Validar formato decimal numérico en strings
        if (isNaN(parseFloat(newDoc.base)) || isNaN(parseFloat(newDoc.iva)) || isNaN(parseFloat(newDoc.total))) {
            throw({ forbidden: "Los importes deben ser representaciones numéricas válidas." });
        }
    }

    // Validar documentos de tipo factura
    if (newDoc.tipo === "factura") {
        if (!newDoc.numero_control_simulado || !newDoc.estado_validacion) {
            throw({ forbidden: "Factura requiere numero_control_simulado y estado_validacion." });
        }
    }

    // Validar declaraciones
    if (newDoc.tipo === "declaracion") {
        if (!newDoc.periodo || !newDoc.estado) {
            throw({ forbidden: "Declaración requiere período y estado." });
        }
    }
}
