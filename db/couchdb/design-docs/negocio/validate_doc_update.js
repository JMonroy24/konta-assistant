function (newDoc, oldDoc, userCtx, secObj) {
  // ES5 puro: el motor JS de CouchDB 3.x (SpiderMonkey 1.8.5) no soporta let/const/flechas.
  var isAdmin = userCtx.roles.indexOf('_admin') !== -1 || userCtx.roles.indexOf('fiscal_admin') !== -1;

  if (newDoc._id.indexOf('_design/') === 0) {
    if (!isAdmin) { throw ({ forbidden: 'Solo administradores modifican design docs' }); }
    return;
  }
  if (newDoc._deleted === true) { return; } // tombstones siempre permitidos (replicacion)

  function fail(m) { throw ({ forbidden: m }); }
  function isStr(v) { return typeof v === 'string' && v.length > 0; }
  function need(f) { if (!isStr(newDoc[f])) { fail('Campo obligatorio (texto no vacio): ' + f); } }
  function oneOf(f, list) {
    if (list.indexOf(newDoc[f]) === -1) { fail(f + ' debe ser uno de: ' + list.join(', ')); }
  }
  function isDate(v) {
    var m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(typeof v === 'string' ? v : '');
    if (!m) { return false; }
    var y = +m[1], mo = +m[2], d = +m[3];
    if (mo < 1 || mo > 12 || d < 1) { return false; }
    var leap = (y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0));
    var dim = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    return d <= dim[mo - 1];
  }
  function isDateTime(v) {
    return typeof v === 'string' &&
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$/.test(v) &&
      isDate(v.substr(0, 10));
  }
  function isMoneyPos(v) { return typeof v === 'string' && /^\d+(\.\d{1,2})?$/.test(v); }
  function cents(s) {
    var p = s.split('.');
    return parseInt(p[0], 10) * 100 + parseInt(((p[1] || '') + '00').substr(0, 2), 10);
  }
  function needDate(f) { if (!isDate(newDoc[f])) { fail(f + ' debe ser fecha ISO valida AAAA-MM-DD'); } }
  function needMoney(f) {
    if (!isMoneyPos(newDoc[f])) { fail(f + ' debe ser cadena decimal con hasta 2 decimales (ej. "113.00")'); }
  }

  var TIPOS = ['negocio', 'venta', 'compra', 'factura', 'declaracion', 'recordatorio'];
  var OBLIG = ['iva_f07', 'pago_cuenta_f14', 'renta_anual_f11'];

  need('tipo');
  oneOf('tipo', TIPOS);
  if (newDoc._id.indexOf(newDoc.tipo + ':') !== 0) {
    fail('_id debe empezar con "' + newDoc.tipo + ':" (UUID generado en el cliente => idempotencia)');
  }
  if (oldDoc && oldDoc.tipo && oldDoc.tipo !== newDoc.tipo) { fail('El campo tipo es inmutable'); }

  switch (newDoc.tipo) {
    case 'venta':
    case 'compra':
      needDate('fecha');
      need('descripcion');
      needMoney('base'); needMoney('iva'); needMoney('total');
      if (cents(newDoc.base) + cents(newDoc.iva) !== cents(newDoc.total)) {
        fail('total debe ser igual a base + iva');
      }
      need('creado_por');
      oneOf('origen', ['local', 'remoto', 'api']);
      if (newDoc.estado !== undefined) { oneOf('estado', ['registrada', 'anulada']); }
      if (newDoc.contraparte !== undefined && typeof newDoc.contraparte !== 'object') {
        fail('contraparte debe ser objeto {nombre, nit}');
      }
      break;

    case 'factura':
      need('numero_control');
      need('venta_id');
      if (newDoc.venta_id.indexOf('venta:') !== 0) { fail('venta_id debe referenciar un documento venta:*'); }
      oneOf('estado', ['pendiente', 'aceptado_simulado', 'rechazado']);
      if (!isDateTime(newDoc.hora_emision)) { fail('hora_emision debe ser fecha-hora ISO 8601 con zona'); }
      if (newDoc.simulado !== true) { fail('simulado debe ser true: no hay conexion real con Hacienda'); }
      if (newDoc.estado === 'aceptado_simulado') { need('sello_simulado'); }
      break;

    case 'declaracion':
      oneOf('obligacion', OBLIG);
      if (!/^\d{4}(-(0[1-9]|1[0-2]))?$/.test(typeof newDoc.periodo === 'string' ? newDoc.periodo : '')) {
        fail('periodo debe ser AAAA o AAAA-MM');
      }
      oneOf('estado', ['borrador', 'presentada']);
      if (typeof newDoc.montos !== 'object' || newDoc.montos === null) { fail('montos debe ser un objeto'); }
      if (newDoc.estado === 'presentada') { needDate('fecha_presentacion'); }
      break;

    case 'recordatorio':
      oneOf('obligacion', OBLIG);
      need('periodo');
      needDate('fecha_limite');
      oneOf('estado', ['pendiente', 'enviado', 'cumplido']);
      break;

    case 'negocio':
      need('nombre');
      oneOf('tipo_contribuyente', ['natural', 'juridica']);
      break;
  }
  // NOTA: no se validan transiciones de estado aqui a proposito: un rechazo durante la
  // replicacion deja el documento atascado. Las transiciones se validan en Rust (M3/M5/M6).
}
