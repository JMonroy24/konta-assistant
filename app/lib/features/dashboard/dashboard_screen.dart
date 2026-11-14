import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../../core/auth_service.dart';
import '../../core/offline_queue.dart';
import '../../core/sync_service.dart';
import '../ventas_compras/models/operacion_model.dart';
import '../ventas_compras/services/operaciones_service.dart';
import '../ventas_compras/widgets/registro_operacion_dialog.dart';

class DashboardScreen extends StatefulWidget {
  const DashboardScreen({
    this.session,
    this.operacionesService,
    this.offlineQueue,
    this.syncService,
    this.onNavegarOperaciones,
    super.key,
  });

  final AuthSession? session;
  final OperacionesService? operacionesService;
  final OfflineQueue? offlineQueue;
  final SyncService? syncService;
  final VoidCallback? onNavegarOperaciones;

  @override
  State<DashboardScreen> createState() => _DashboardScreenState();
}

class _DashboardScreenState extends State<DashboardScreen> {
  late final OperacionesService _operacionesService;
  late final OfflineQueue _offlineQueue;
  late final SyncService _syncService;

  bool _isLoading = true;
  bool _isSyncing = false;
  List<OperacionModel> _operaciones = [];
  int _pendingCount = 0;

  final String _periodoActual = DateFormat('yyyy-MM').format(DateTime.now());

  @override
  void initState() {
    super.initState();
    _offlineQueue = widget.offlineQueue ?? OfflineQueue();
    _operacionesService = widget.operacionesService ?? OperacionesService(offlineQueue: _offlineQueue);
    _syncService = widget.syncService ?? SyncService(offlineQueue: _offlineQueue);
    _cargarDatos();
  }

  Future<void> _cargarDatos() async {
    setState(() => _isLoading = true);
    await _offlineQueue.init();

    try {
      final ops = await _operacionesService.listarOperaciones(
        periodoMes: _periodoActual,
        authToken: widget.session?.token,
      );
      if (!mounted) return;
      setState(() {
        _operaciones = ops;
        _pendingCount = _offlineQueue.pendingCount;
        _isLoading = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _pendingCount = _offlineQueue.pendingCount;
        _isLoading = false;
      });
    }
  }

  Future<void> _forzarSincronizacion() async {
    if (_isSyncing) return;
    setState(() => _isSyncing = true);

    final resultado = await _syncService.syncPending(authToken: widget.session?.token);

    if (!mounted) return;
    setState(() {
      _isSyncing = false;
      _pendingCount = _offlineQueue.pendingCount;
    });

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(resultado.mensaje),
        backgroundColor: resultado.estadoFinal == SyncState.success ||
                resultado.estadoFinal == SyncState.idle
            ? Colors.green.shade800
            : Colors.orange.shade800,
        behavior: SnackBarBehavior.floating,
      ),
    );

    await _cargarDatos();
  }

  void _abrirRegistro(TipoOperacion tipo) async {
    final res = await RegistroOperacionDialog.show(
      context,
      operacionesService: _operacionesService,
      creadoPor: widget.session?.username ?? 'usuario',
      authToken: widget.session?.token,
      tipoInicial: tipo,
    );
    if (res != null) {
      await _cargarDatos();
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final resumen = _operacionesService.calcularResumen(_operaciones);

    // Tasa de pago a cuenta 1.75% mensual sobre ingresos brutos (El Salvador)
    final double pagoACuenta = OperacionModel.redondear2Decimales(resumen.totalVentas * 0.0175);
    final bool tieneRemanente = resumen.creditoIva > resumen.debitoIva;
    final double ivaAPagar = tieneRemanente ? 0.0 : resumen.ivaNeto;
    final double remanenteAFavor = tieneRemanente ? OperacionModel.redondear2Decimales(resumen.creditoIva - resumen.debitoIva) : 0.0;

    final String mesNombre = DateFormat('MMMM yyyy', 'es').format(DateTime.now());

    return RefreshIndicator(
      onRefresh: _cargarDatos,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Cabecera de Bienvenida
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Hola, ${widget.session?.username ?? 'Contribuyente'}',
                      style: Theme.of(context).textTheme.titleLarge?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      'Período tributario activo: $mesNombre ($_periodoActual)',
                      style: TextStyle(fontSize: 13, color: Colors.grey.shade700),
                    ),
                  ],
                ),
                IconButton(
                  icon: const Icon(Icons.refresh),
                  tooltip: 'Actualizar resumen',
                  onPressed: _cargarDatos,
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Panel de Estado de Sincronización
            Card(
              elevation: 0,
              color: _pendingCount > 0 ? Colors.amber.shade50 : Colors.green.shade50,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
                side: BorderSide(
                  color: _pendingCount > 0 ? Colors.amber.shade300 : Colors.green.shade200,
                ),
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                child: Row(
                  children: [
                    CircleAvatar(
                      backgroundColor: _pendingCount > 0 ? Colors.amber.shade100 : Colors.green.shade100,
                      child: Icon(
                        _pendingCount > 0 ? Icons.cloud_queue : Icons.cloud_done,
                        color: _pendingCount > 0 ? Colors.amber.shade900 : Colors.green.shade900,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            _pendingCount > 0
                                ? '$_pendingCount operaciones pendientes en cola local'
                                : 'Todos los documentos sincronizados',
                            style: TextStyle(
                              fontWeight: FontWeight.bold,
                              color: _pendingCount > 0 ? Colors.amber.shade900 : Colors.green.shade900,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            _pendingCount > 0
                                ? 'Se enviarán automáticamente cuando haya conexión estable.'
                                : 'Base de datos remota CouchDB actualizada.',
                            style: TextStyle(fontSize: 12, color: Colors.grey.shade700),
                          ),
                        ],
                      ),
                    ),
                    if (_pendingCount > 0)
                      FilledButton.tonal(
                        onPressed: _isSyncing ? null : _forzarSincronizacion,
                        child: _isSyncing
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(strokeWidth: 2),
                              )
                            : const Text('Sincronizar'),
                      ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 20),

            // Tarjetas de Resumen Fiscal Principal
            const Text(
              'Obligaciones Estimadas del Mes',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 12),

            Row(
              children: [
                // Tarjeta IVA
                Expanded(
                  child: Card(
                    elevation: 1,
                    color: tieneRemanente ? Colors.green.shade50 : Colors.blue.shade50,
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                    child: Padding(
                      padding: const EdgeInsets.all(16.0),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text(
                                tieneRemanente ? 'Remanente IVA' : 'IVA a Pagar',
                                style: TextStyle(
                                  color: tieneRemanente ? Colors.green.shade900 : Colors.blue.shade900,
                                  fontWeight: FontWeight.w600,
                                  fontSize: 13,
                                ),
                              ),
                              Icon(
                                tieneRemanente ? Icons.arrow_downward : Icons.account_balance_wallet_outlined,
                                size: 18,
                                color: tieneRemanente ? Colors.green.shade700 : Colors.blue.shade700,
                              ),
                            ],
                          ),
                          const SizedBox(height: 8),
                          Text(
                            tieneRemanente ? '\$${remanenteAFavor.toStringAsFixed(2)}' : '\$${ivaAPagar.toStringAsFixed(2)}',
                            style: TextStyle(
                              fontSize: 24,
                              fontWeight: FontWeight.bold,
                              color: tieneRemanente ? Colors.green.shade900 : Colors.blue.shade900,
                            ),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            tieneRemanente ? 'A favor próximo período' : 'Declaración mensual F-07',
                            style: TextStyle(fontSize: 11, color: Colors.grey.shade600),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 12),

                // Tarjeta Pago a Cuenta
                Expanded(
                  child: Card(
                    elevation: 1,
                    color: Colors.teal.shade50,
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                    child: Padding(
                      padding: const EdgeInsets.all(16.0),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text(
                                'Pago a Cuenta (1.75%)',
                                style: TextStyle(
                                  color: Colors.teal.shade900,
                                  fontWeight: FontWeight.w600,
                                  fontSize: 13,
                                ),
                              ),
                              Icon(Icons.percent, size: 18, color: Colors.teal.shade700),
                            ],
                          ),
                          const SizedBox(height: 8),
                          Text(
                            '\$${pagoACuenta.toStringAsFixed(2)}',
                            style: TextStyle(
                              fontSize: 24,
                              fontWeight: FontWeight.bold,
                              color: Colors.teal.shade900,
                            ),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            'Anticipo de renta mensual',
                            style: TextStyle(fontSize: 11, color: Colors.grey.shade600),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Resumen de Movimientos Netos
            Card(
              elevation: 0,
              color: colors.surfaceContainerHighest.withValues(alpha: 0.35),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Total Ventas Netas (Ingresos):'),
                        Text(
                          '\$${resumen.totalVentas.toStringAsFixed(2)}',
                          style: const TextStyle(fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Total Compras Netas (Gastos):'),
                        Text(
                          '\$${resumen.totalCompras.toStringAsFixed(2)}',
                          style: const TextStyle(fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                    const Divider(height: 20),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Débito Fiscal generado (IVA Ventas):'),
                        Text(
                          '\$${resumen.debitoIva.toStringAsFixed(2)}',
                          style: TextStyle(color: Colors.blue.shade800, fontWeight: FontWeight.w600),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Crédito Fiscal deducible (IVA Compras):'),
                        Text(
                          '\$${resumen.creditoIva.toStringAsFixed(2)}',
                          style: TextStyle(color: Colors.teal.shade800, fontWeight: FontWeight.w600),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 20),

            // Explicación en lenguaje simple para la microempresa
            Card(
              elevation: 0,
              color: Colors.indigo.shade50.withValues(alpha: 0.6),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
                side: BorderSide(color: Colors.indigo.shade100),
              ),
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(Icons.lightbulb_outline, color: Colors.indigo.shade800, size: 20),
                        const SizedBox(width: 8),
                        Text(
                          'Explicación en lenguaje simple:',
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            color: Colors.indigo.shade900,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    Text(
                      resumen.cantidadOperaciones == 0
                          ? 'Aún no has registrado operaciones en este mes. Agrega tus compras y ventas con el botón inferior para calcular automáticamente tus impuestos.'
                          : tieneRemanente
                              ? 'Tus compras del mes generaron \$${resumen.creditoIva.toStringAsFixed(2)} de crédito fiscal, superando los \$${resumen.debitoIva.toStringAsFixed(2)} de débito fiscal de tus ventas. Tienes un remanente a tu favor de \$${remanenteAFavor.toStringAsFixed(2)} para descontar el próximo mes. Tu anticipo de pago a cuenta es de \$${pagoACuenta.toStringAsFixed(2)}.'
                              : 'Tus ventas generaron \$${resumen.debitoIva.toStringAsFixed(2)} de IVA débito y tus compras acumularon \$${resumen.creditoIva.toStringAsFixed(2)} de crédito fiscal. La diferencia líquida a pagar a Hacienda en el formulario F-07 es de \$${ivaAPagar.toStringAsFixed(2)}, más \$${pagoACuenta.toStringAsFixed(2)} de anticipo a cuenta.',
                      style: const TextStyle(fontSize: 13, height: 1.4),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 24),

            // Acciones Rápidas
            const Text(
              'Acciones Rápidas',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: OutlinedButton.icon(
                    style: OutlinedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      side: BorderSide(color: colors.primary),
                    ),
                    onPressed: () => _abrirRegistro(TipoOperacion.venta),
                    icon: const Icon(Icons.arrow_upward),
                    label: const Text('Registrar Venta'),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: OutlinedButton.icon(
                    style: OutlinedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(vertical: 14),
                      side: BorderSide(color: colors.secondary),
                    ),
                    onPressed: () => _abrirRegistro(TipoOperacion.compra),
                    icon: const Icon(Icons.arrow_downward),
                    label: const Text('Registrar Compra'),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
