function (newDoc, oldDoc, userCtx, secObj) {
  var isAdmin = userCtx.roles.indexOf('_admin') !== -1 || userCtx.roles.indexOf('fiscal_admin') !== -1;
  if (!isAdmin) { throw ({ forbidden: 'reglas_fiscales solo la modifica un administrador fiscal' }); }
  if (newDoc._deleted === true) { return; }
  if (newDoc._id.indexOf('_design/') === 0) { return; }

  function fail(m) { throw ({ forbidden: m }); }
  function isDate(v) { return typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v); }

  if (newDoc.tipo !== 'regla_fiscal') { fail('tipo debe ser regla_fiscal'); }
  if (typeof newDoc.codigo !== 'string' || !/^[a-z0-9_]+$/.test(newDoc.codigo)) { fail('codigo invalido (a-z, 0-9, _)'); }
  if (typeof newDoc.version !== 'number' || newDoc.version < 1 || newDoc.version % 1 !== 0) { fail('version debe ser entero >= 1'); }
  if (typeof newDoc.parametros !== 'object' || newDoc.parametros === null) { fail('parametros debe ser objeto'); }
  if (!isDate(newDoc.vigencia_desde)) { fail('vigencia_desde debe ser AAAA-MM-DD'); }
  if (newDoc.vigencia_hasta !== null && newDoc.vigencia_hasta !== undefined) {
    if (!isDate(newDoc.vigencia_hasta)) { fail('vigencia_hasta debe ser AAAA-MM-DD o null'); }
    if (newDoc.vigencia_hasta < newDoc.vigencia_desde) { fail('vigencia_hasta < vigencia_desde'); }
  }
  if (typeof newDoc.fuente_normativa !== 'string' || newDoc.fuente_normativa.length < 5) { fail('fuente_normativa obligatoria'); }
  if (typeof newDoc.verificada !== 'boolean') { fail('verificada (boolean) obligatoria: indica si se contrasto con Hacienda'); }
  if (newDoc._id !== 'regla:' + newDoc.codigo + ':v' + newDoc.version) { fail('_id debe ser regla:<codigo>:v<version>'); }
}
