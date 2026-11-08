function (doc) {
  // Importes como CENTAVOS ENTEROS (nunca float): el reduce _sum suma arreglos elemento a elemento.
  function cents(s) {
    var m = /^(\d+)(?:\.(\d{1,2}))?$/.exec(s);
    if (!m) { return 0; }
    return parseInt(m[1], 10) * 100 + parseInt(((m[2] || '') + '00').substr(0, 2), 10);
  }
  if ((doc.tipo === 'venta' || doc.tipo === 'compra') && doc.estado !== 'anulada' && doc.fecha) {
    // clave: [AAAA-MM, tipo]; valor: [base, iva, total] en centavos
    emit([doc.fecha.substr(0, 7), doc.tipo], [cents(doc.base), cents(doc.iva), cents(doc.total)]);
  }
}
