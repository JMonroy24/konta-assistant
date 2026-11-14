import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../../core/auth_service.dart';
import 'models/operacion_model.dart';
import 'services/operaciones_service.dart';
import 'widgets/registro_operacion_dialog.dart';

class VentasComprasScreen extends StatefulWidget {
  const VentasComprasScreen({
    this.session,
    this.operacionesService,
    super.key,
  });

  final AuthSession? session;
  final OperacionesService? operacionesService;

  @override
  State<VentasComprasScreen> createState() => _VentasComprasScreenState();
}

class _VentasComprasScreenState extends State<VentasComprasScreen>
    with SingleTickerProviderStateMixin {
  late final OperacionesService _operacionesService;
  late TabController _tabController;

  DateTime _mesSeleccionado = DateTime.now();
  List<OperacionModel> _operaciones = [];
  bool _isLoading = true;
  String? _errorMessage;
  String _filtroTexto = '';

  @override
  void initState() {
    super.initState();
    _operacionesService = widget.operacionesService ?? OperacionesService();
    _tabController = TabController(length: 3, vsync: this);
    _tabController.addListener(() {
      if (mounted) setState(() {});
    });
    _cargarOperaciones();
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  String get _periodoFormateado => DateFormat('yyyy-MM').format(_mesSeleccionado);
  String get _mesNombreFormateado => DateFormat('MMMM yyyy', 'es').format(_mesSeleccionado);

  Future<void> _cargarOperaciones() async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final list = await _operacionesService.listarOperaciones(
        periodoMes: _periodoFormateado,
        authToken: widget.session?.token,
      );
      if (!mounted) return;
      setState(() {
        _operaciones = list;
        _isLoading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _errorMessage = 'Error al cargar operaciones: $e';
        _isLoading = false;
      });
    }
  }

  void _mesAnterior() {
    setState(() {
      _mesSeleccionado = DateTime(_mesSeleccionado.year, _mesSeleccionado.month - 1);
    });
    _cargarOperaciones();
  }

  void _mesSiguiente() {
    setState(() {
      _mesSeleccionado = DateTime(_mesSeleccionado.year, _mesSeleccionado.month + 1);
    });
    _cargarOperaciones();
  }

  void _abrirFormularioRegistro({TipoOperacion tipo = TipoOperacion.venta}) async {
    final nueva = await RegistroOperacionDialog.show(
      context,
      operacionesService: _operacionesService,
      creadoPor: widget.session?.username ?? 'usuario_local',
      authToken: widget.session?.token,
      tipoInicial: tipo,
    );

    if (nueva != null) {
      _cargarOperaciones();
    }
  }

  void _mostrarDetalleOperacion(OperacionModel op) {
    final colors = Theme.of(context).colorScheme;
    final esVenta = op.tipo == TipoOperacion.venta;

    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (context) {
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      CircleAvatar(
                        backgroundColor: esVenta ? Colors.blue.shade100 : Colors.teal.shade100,
                        child: Icon(
                          esVenta ? Icons.arrow_upward : Icons.arrow_downward,
                          color: esVenta ? Colors.blue.shade900 : Colors.teal.shade900,
                        ),
                      ),
                      const SizedBox(width: 12),
                      Text(
                        'Detalle de ${op.tipo.displayName}',
                        style: Theme.of(context).textTheme.titleLarge?.copyWith(
                              fontWeight: FontWeight.bold,
                            ),
                      ),
                    ],
                  ),
                  if (op.isPendingSync)
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: Colors.orange.shade100,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Row(
                        children: [
                          Icon(Icons.cloud_queue, size: 14, color: Colors.orange.shade900),
                          const SizedBox(width: 4),
                          Text(
                            'En cola offline',
                            style: TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.bold,
                              color: Colors.orange.shade900,
                            ),
                          ),
                        ],
                      ),
                    ),
                ],
              ),
              const Divider(height: 24),
              _itemDetalle('ID de Documento', op.id),
              _itemDetalle('Fecha', DateFormat('dd/MM/yyyy').format(op.fecha)),
              _itemDetalle(op.tipo.contraparteLabel, op.contraparte),
              _itemDetalle('Descripción', op.descripcion),
              _itemDetalle('Base Imponible', '\$${op.base.toStringAsFixed(2)}'),
              _itemDetalle('IVA (13%)', '\$${op.iva.toStringAsFixed(2)}'),
              _itemDetalle(
                'Total',
                '\$${op.total.toStringAsFixed(2)}',
                isBold: true,
                color: esVenta ? colors.primary : colors.secondary,
              ),
              _itemDetalle('Registrado por', op.creadoPor),
              _itemDetalle('Origen', op.origen),
              const SizedBox(height: 16),
              Align(
                alignment: Alignment.centerRight,
                child: FilledButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('Cerrar'),
                ),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _itemDetalle(String label, String value, {bool isBold = false, Color? color}) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.black54)),
          Flexible(
            child: Text(
              value,
              textAlign: TextAlign.right,
              style: TextStyle(
                fontWeight: isBold ? FontWeight.bold : FontWeight.w500,
                color: color,
              ),
            ),
          ),
        ],
      ),
    );
  }

  List<OperacionModel> get _operacionesFiltradas {
    return _operaciones.where((op) {
      // Filtro por pestaña
      if (_tabController.index == 1 && op.tipo != TipoOperacion.venta) return false;
      if (_tabController.index == 2 && op.tipo != TipoOperacion.compra) return false;

      // Filtro por texto
      if (_filtroTexto.isNotEmpty) {
        final query = _filtroTexto.toLowerCase();
        final matchDesc = op.descripcion.toLowerCase().contains(query);
        final matchContra = op.contraparte.toLowerCase().contains(query);
        return matchDesc || matchContra;
      }
      return true;
    }).toList();
  }

  @override
  Widget build(BuildContext context) {
    final resumen = _operacionesService.calcularResumen(_operaciones);

    return Scaffold(
      body: RefreshIndicator(
        onRefresh: _cargarOperaciones,
        child: Column(
          children: [
            // Barra de Período Mensual
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: Theme.of(context).colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  IconButton(
                    icon: const Icon(Icons.chevron_left),
                    tooltip: 'Mes anterior',
                    onPressed: _mesAnterior,
                  ),
                  Row(
                    children: [
                      const Icon(Icons.calendar_month, size: 20),
                      const SizedBox(width: 8),
                      Text(
                        _periodoFormateado,
                        style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
                      ),
                    ],
                  ),
                  IconButton(
                    icon: const Icon(Icons.chevron_right),
                    tooltip: 'Mes siguiente',
                    onPressed: _mesSiguiente,
                  ),
                ],
              ),
            ),

            // Tarjeta de Resumen Rápido de Totales
            Padding(
              padding: const EdgeInsets.all(12),
              child: Card(
                elevation: 0,
                color: Theme.of(context).colorScheme.primaryContainer.withValues(alpha: 0.25),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceAround,
                    children: [
                      _kpiItem('Ventas', '\$${resumen.totalVentas.toStringAsFixed(2)}', Colors.blue.shade800),
                      _kpiItem('Compras', '\$${resumen.totalCompras.toStringAsFixed(2)}', Colors.teal.shade800),
                      _kpiItem('Débito IVA', '\$${resumen.debitoIva.toStringAsFixed(2)}', Colors.indigo.shade800),
                      _kpiItem('Crédito IVA', '\$${resumen.creditoIva.toStringAsFixed(2)}', Colors.deepOrange.shade800),
                    ],
                  ),
                ),
              ),
            ),

            // Pestañas de Filtro (Todas / Ventas / Compras)
            TabBar(
              controller: _tabController,
              labelColor: Theme.of(context).colorScheme.primary,
              indicatorColor: Theme.of(context).colorScheme.primary,
              tabs: const [
                Tab(text: 'Todas'),
                Tab(text: 'Ventas'),
                Tab(text: 'Compras'),
              ],
            ),

            // Buscador rápido
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: TextField(
                decoration: InputDecoration(
                  hintText: 'Buscar por descripción o cliente/proveedor...',
                  prefixIcon: const Icon(Icons.search, size: 20),
                  contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 0),
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
                  filled: true,
                  fillColor: Colors.grey.shade50,
                ),
                onChanged: (val) => setState(() => _filtroTexto = val.trim()),
              ),
            ),

            // Lista de Operaciones
            Expanded(
              child: _isLoading
                  ? const Center(child: CircularProgressIndicator())
                  : _errorMessage != null
                      ? Center(
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Icon(Icons.error_outline, size: 48, color: Colors.red.shade300),
                              const SizedBox(height: 12),
                              Text(_errorMessage!),
                              const SizedBox(height: 12),
                              ElevatedButton(
                                onPressed: _cargarOperaciones,
                                child: const Text('Reintentar'),
                              ),
                            ],
                          ),
                        )
                      : _operacionesFiltradas.isEmpty
                          ? Center(
                              child: Column(
                                mainAxisAlignment: MainAxisAlignment.center,
                                children: [
                                  Icon(Icons.receipt_long_outlined, size: 54, color: Colors.grey.shade400),
                                  const SizedBox(height: 12),
                                  Text(
                                    'No hay operaciones registradas en $_periodoFormateado',
                                    style: TextStyle(color: Colors.grey.shade600),
                                  ),
                                  const SizedBox(height: 8),
                                  TextButton.icon(
                                    onPressed: () => _abrirFormularioRegistro(),
                                    icon: const Icon(Icons.add),
                                    label: const Text('Registrar la primera operación'),
                                  ),
                                ],
                              ),
                            )
                          : ListView.builder(
                              itemCount: _operacionesFiltradas.length,
                              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                              itemBuilder: (context, index) {
                                final op = _operacionesFiltradas[index];
                                final esVenta = op.tipo == TipoOperacion.venta;
                                final color = esVenta ? Colors.blue.shade700 : Colors.teal.shade700;

                                return Card(
                                  elevation: 1,
                                  margin: const EdgeInsets.symmetric(vertical: 4),
                                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
                                  child: ListTile(
                                    onTap: () => _mostrarDetalleOperacion(op),
                                    leading: CircleAvatar(
                                      backgroundColor: color.withValues(alpha: 0.12),
                                      child: Icon(
                                        esVenta ? Icons.arrow_upward : Icons.arrow_downward,
                                        color: color,
                                        size: 20,
                                      ),
                                    ),
                                    title: Row(
                                      children: [
                                        Expanded(
                                          child: Text(
                                            op.contraparte,
                                            style: const TextStyle(fontWeight: FontWeight.bold),
                                            overflow: TextOverflow.ellipsis,
                                          ),
                                        ),
                                        if (op.isPendingSync)
                                          Tooltip(
                                            message: 'Guardado localmente en cola offline',
                                            child: Icon(
                                              Icons.cloud_queue,
                                              size: 16,
                                              color: Colors.orange.shade800,
                                            ),
                                          ),
                                      ],
                                    ),
                                    subtitle: Text(
                                      '${DateFormat('dd/MM/yyyy').format(op.fecha)} • ${op.descripcion}\nBase: \$${op.base.toStringAsFixed(2)} | IVA: \$${op.iva.toStringAsFixed(2)}',
                                      style: const TextStyle(fontSize: 12),
                                    ),
                                    trailing: Text(
                                      '\$${op.total.toStringAsFixed(2)}',
                                      style: TextStyle(
                                        fontWeight: FontWeight.bold,
                                        fontSize: 15,
                                        color: color,
                                      ),
                                    ),
                                  ),
                                );
                              },
                            ),
            ),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => _abrirFormularioRegistro(),
        icon: const Icon(Icons.add),
        label: const Text('Nueva Operación'),
      ),
    );
  }

  Widget _kpiItem(String label, String value, Color color) {
    return Column(
      children: [
        Text(label, style: const TextStyle(fontSize: 11, color: Colors.black54)),
        const SizedBox(height: 2),
        Text(
          value,
          style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13, color: color),
        ),
      ],
    );
  }
}
