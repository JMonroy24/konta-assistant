function (newDoc, oldDoc, userCtx, secObj) {
  if (newDoc._id.indexOf('_design/') === 0) { return; }
  if (newDoc._deleted === true) { return; }
  function fail(m) { throw ({ forbidden: m }); }
  if (newDoc.tipo !== 'usuario') { fail('tipo debe ser usuario'); }
  if (typeof newDoc.email !== 'string' || newDoc.email !== newDoc.email.toLowerCase() || newDoc.email.indexOf('@') < 1) { fail('email invalido'); }
  if (newDoc._id !== 'usuario:' + newDoc.email) { fail('_id debe ser usuario:<email en minusculas>'); }
  if (typeof newDoc.password_hash !== 'string' || newDoc.password_hash.indexOf('$argon2') !== 0) { fail('password_hash debe ser un hash argon2 (PHC)'); }
  if (['administrador', 'contador', 'operador', 'lectura'].indexOf(newDoc.rol) === -1) { fail('rol invalido'); }
  if (!(newDoc.negocios instanceof Array) || newDoc.negocios.length === 0) { fail('negocios debe ser un arreglo no vacio'); }
  if (typeof newDoc.activo !== 'boolean') { fail('activo (boolean) obligatorio'); }
}
