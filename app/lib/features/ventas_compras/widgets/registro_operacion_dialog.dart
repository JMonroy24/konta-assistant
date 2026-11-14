import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../models/operacion_model.dart';
import '../services/operaciones_service.dart';

class RegistroOperacionDialog extends StatefulWidget {
  const RegistroOperacionDialog({
    required this.operacionesService,
    required this.creadoPor,
    this.authToken,
    this.tipoInicial = TipoOperacion.venta,
    this.onOperacionCreada,
    super.key,
  });

  final OperacionesService operacionesService;
  final String creadoPor;
  final String? authToken;
  final TipoOperacion tipoInicial;
  final ValueChanged<OperacionModel>? onOperacionCreada;

  static Future<OperacionModel?> show(
    BuildContext context, {
    required OperacionesService operacionesService,
    required String creadoPor,
    String? authToken,
    TipoOperacion tipoInicial = TipoOperacion.venta,
  }) {
    return showDialog<OperacionModel>(
      context: context,
      barrierDismissible: false,
      builder: (context) => RegistroOperacionDialog(
        operacionesService: operacionesService,
        creadoPor: creadoPor,
        authToken: authToken,
        tipoInicial: tipoInicial,
      ),
    );
  }

  @override
  State<RegistroOperacionDialog> createState() => _RegistroOperacionDialogState();
}

class _RegistroOperacionDialogState extends State<RegistroOperacionDialog> {
  final _formKey = GlobalKey<FormState>();

  late TipoOperacion _tipo;
  late DateTime _fecha;
  final _contraparteController = TextEditingController();
  final _descripcionController = TextEditingController();
  final _montoController = TextEditingController();

  bool _montoIncluyeIva = false;
  double _baseCalculada = 0.0;
  double _ivaCalculado = 0.0;
  double _totalCalculado = 0.0;
  String? _notaAjuste;

  bool _isSaving = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _tipo = widget.tipoInicial;
    _fecha = DateTime.now();
  }

  @override
  void dispose() {
    _contraparteController.dispose();
    _descripcionController.dispose();
    _montoController.dispose();
    super.dispose();
  }

  void _recalcularValores() {
    final monto = MontoParser.parse(_montoController.text);

    if (monto == null || monto <= 0) {
      setState(() {
        _baseCalculada = 0.0;
        _ivaCalculado = 0.0;
        _totalCalculado = 0.0;
        _notaAjuste = null;
      });
      return;
    }

    if (_montoIncluyeIva) {
      final desglose = OperacionModel.desglosarTotalConIva(monto);
      setState(() {
        _baseCalculada = desglose.base;
        _ivaCalculado = desglose.iva;
        _totalCalculado = desglose.total;
        _notaAjuste = desglose.huboAjuste
            ? 'Ajuste fiscal exacto: Base \$${desglose.base.toStringAsFixed(2)} + IVA \$${desglose.iva.toStringAsFixed(2)} = \$${desglose.total.toStringAsFixed(2)} (ajuste de ${(desglose.ajusteCentavos * 100).round()}¢ por redondeo legal a 2 decimales).'
            : null;
      });
    } else {
      final base = OperacionModel.redondear2Decimales(monto);
      final iva = OperacionModel.calcularIva(base);
      final total = OperacionModel.redondear2Decimales(base + iva);
      setState(() {
        _baseCalculada = base;
        _ivaCalculado = iva;
        _totalCalculado = total;
        _notaAjuste = null;
      });
    }
  }

  Future<void> _seleccionarFecha() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _fecha,
      firstDate: DateTime(2020),
      lastDate: DateTime(2030),
      helpText: 'Seleccionar fecha de la operación',
    );
    if (picked != null) {
      setState(() => _fecha = picked);
    }
  }

  Future<void> _guardar() async {
    // Protección estricta contra pulsaciones múltiples (doble submit)
    if (_isSaving) return;

    FocusScope.of(context).unfocus();
    if (!_formKey.currentState!.validate()) return;

    if (_baseCalculada <= 0 || _totalCalculado <= 0) {
      setState(() => _errorMessage = 'El importe ingresado debe ser mayor a \$0.00');
      return;
    }

    setState(() {
      _isSaving = true;
      _errorMessage = null;
    });

    try {
      final nuevaOperacion = OperacionModel.crear(
        tipo: _tipo,
        fecha: _fecha,
        descripcion: _descripcionController.text.trim(),
        base: _baseCalculada,
        contraparte: _contraparteController.text.trim(),
        creadoPor: widget.creadoPor.trim().isEmpty ? 'usuario' : widget.creadoPor.trim(),
      );

      final guardada = await widget.operacionesService.registrarOperacion(
        nuevaOperacion,
        authToken: widget.authToken,
      );

      if (!mounted) return;

      widget.onOperacionCreada?.call(guardada);
      Navigator.of(context).pop(guardada);

      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Row(
            children: [
              Icon(
                guardada.isPendingSync ? Icons.cloud_queue : Icons.check_circle,
                color: Colors.white,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  guardada.isPendingSync
                      ? '${_tipo.displayName} guardada localmente (pendiente de sincronización).'
                      : '${_tipo.displayName} registrada exitosamente en el servidor.',
                ),
              ),
            ],
          ),
          backgroundColor: guardada.isPendingSync ? Colors.orange.shade800 : Colors.green.shade800,
          behavior: SnackBarBehavior.floating,
        ),
      );
    } catch (e) {
      if (!mounted) return;
      setState(() => _errorMessage = 'Error al registrar operación: $e');
    } finally {
      if (mounted) setState(() => _isSaving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final esVenta = _tipo == TipoOperacion.venta;
    final colorTipo = esVenta ? Colors.blue.shade700 : Colors.teal.shade700;

    return Dialog(
      insetPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 24),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500),
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(20),
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Cabecera
                Row(
                  children: [
                    CircleAvatar(
                      backgroundColor: colorTipo.withValues(alpha: 0.15),
                      child: Icon(
                        esVenta ? Icons.arrow_upward : Icons.arrow_downward,
                        color: colorTipo,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        'Registrar ${esVenta ? 'Venta' : 'Compra'}',
                        style: Theme.of(context).textTheme.titleLarge?.copyWith(
                              fontWeight: FontWeight.bold,
                              color: colorTipo,
                            ),
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.close),
                      onPressed: _isSaving ? null : () => Navigator.of(context).pop(),
                    ),
                  ],
                ),
                const SizedBox(height: 16),

                // Selector de Tipo de Operación
                SegmentedButton<TipoOperacion>(
                  segments: const [
                    ButtonSegment(
                      value: TipoOperacion.venta,
                      label: Text('Venta (Ingreso)'),
                      icon: Icon(Icons.receipt_long),
                    ),
                    ButtonSegment(
                      value: TipoOperacion.compra,
                      label: Text('Compra (Gasto)'),
                      icon: Icon(Icons.shopping_bag_outlined),
                    ),
                  ],
                  selected: {_tipo},
                  onSelectionChanged: (set) {
                    setState(() {
                      _tipo = set.first;
                    });
                  },
                ),
                const SizedBox(height: 16),

                // Fecha
                InkWell(
                  onTap: _isSaving ? null : _seleccionarFecha,
                  borderRadius: BorderRadius.circular(8),
                  child: InputDecorator(
                    decoration: const InputDecoration(
                      labelText: 'Fecha de Operación',
                      prefixIcon: Icon(Icons.calendar_today_outlined),
                      border: OutlineInputBorder(),
                    ),
                    child: Text(
                      DateFormat('dd / MM / yyyy').format(_fecha),
                      style: const TextStyle(fontWeight: FontWeight.w500),
                    ),
                  ),
                ),
                const SizedBox(height: 14),

                // Contraparte (Cliente / Proveedor)
                TextFormField(
                  controller: _contraparteController,
                  enabled: !_isSaving,
                  decoration: InputDecoration(
                    labelText: _tipo.contraparteLabel,
                    hintText: esVenta ? 'Ej. Distribuidora San Juan' : 'Ej. Ferretería El Sol',
                    prefixIcon: Icon(esVenta ? Icons.person_outline : Icons.store_outlined),
                    border: const OutlineInputBorder(),
                  ),
                  validator: (val) {
                    if (val == null || val.trim().isEmpty) {
                      return 'Por favor ingresa el ${_tipo.contraparteLabel.toLowerCase()}.';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 14),

                // Descripción
                TextFormField(
                  controller: _descripcionController,
                  enabled: !_isSaving,
                  decoration: InputDecoration(
                    labelText: 'Descripción del Bien o Servicio',
                    hintText: esVenta ? 'Ej. Venta de artículos varios' : 'Ej. Compra de inventario',
                    prefixIcon: const Icon(Icons.description_outlined),
                    border: const OutlineInputBorder(),
                  ),
                  validator: (val) {
                    if (val == null || val.trim().isEmpty) {
                      return 'La descripción es obligatoria.';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 14),

                // Toggle si monto incluye IVA
                SwitchListTile(
                  contentPadding: EdgeInsets.zero,
                  title: const Text('¿El importe ingresado incluye IVA (13%)?'),
                  subtitle: Text(
                    _montoIncluyeIva
                        ? 'Se calculará la Base Imponible legal dividiendo entre 1.13'
                        : 'Se calculará el IVA sumando el 13% a la Base',
                    style: TextStyle(fontSize: 12, color: Colors.grey.shade600),
                  ),
                  value: _montoIncluyeIva,
                  onChanged: _isSaving
                      ? null
                      : (val) {
                          setState(() => _montoIncluyeIva = val);
                          _recalcularValores();
                        },
                ),
                const SizedBox(height: 8),

                // Campo Importe / Base (soporta comas, puntos y símbolos)
                TextFormField(
                  controller: _montoController,
                  enabled: !_isSaving,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: InputDecoration(
                    labelText: _montoIncluyeIva ? 'Total con IVA (\$) *' : 'Base Imponible (\$) *',
                    hintText: '0.00',
                    prefixIcon: const Icon(Icons.attach_money),
                    border: const OutlineInputBorder(),
                  ),
                  onChanged: (_) => _recalcularValores(),
                  validator: (val) {
                    if (val == null || val.trim().isEmpty) {
                      return 'Ingresa el monto de la operación.';
                    }
                    final parsed = MontoParser.parse(val);
                    if (parsed == null || parsed <= 0) {
                      return 'Ingresa un valor numérico válido mayor a \$0.00.';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 16),

                // Tarjeta de Desglose Fiscal
                Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: colors.surfaceContainerHighest.withValues(alpha: 0.4),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: Colors.grey.shade300),
                  ),
                  child: Column(
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          const Text('Base Imponible:'),
                          Text(
                            '\$${_baseCalculada.toStringAsFixed(2)}',
                            style: const TextStyle(fontWeight: FontWeight.w600),
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Text('IVA Crédito/Débito (13%):', style: TextStyle(color: colorTipo)),
                          Text(
                            '\$${_ivaCalculado.toStringAsFixed(2)}',
                            style: TextStyle(fontWeight: FontWeight.w600, color: colorTipo),
                          ),
                        ],
                      ),
                      const Divider(height: 16),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          const Text(
                            'Total a Pagar / Percibir:',
                            style: TextStyle(fontWeight: FontWeight.bold, fontSize: 15),
                          ),
                          Text(
                            '\$${_totalCalculado.toStringAsFixed(2)}',
                            style: TextStyle(
                              fontWeight: FontWeight.bold,
                              fontSize: 16,
                              color: colorTipo,
                            ),
                          ),
                        ],
                      ),
                      if (_notaAjuste != null) ...[
                        const SizedBox(height: 8),
                        Text(
                          _notaAjuste!,
                          style: TextStyle(
                            fontSize: 11,
                            color: Colors.amber.shade900,
                            fontStyle: FontStyle.italic,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),

                if (_errorMessage != null) ...[
                  const SizedBox(height: 14),
                  Container(
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: Colors.red.shade50,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: Colors.red.shade200),
                    ),
                    child: Text(
                      _errorMessage!,
                      style: TextStyle(color: Colors.red.shade800, fontSize: 13),
                    ),
                  ),
                ],

                const SizedBox(height: 20),

                // Botones de acción
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    TextButton(
                      onPressed: _isSaving ? null : () => Navigator.of(context).pop(),
                      child: const Text('Cancelar'),
                    ),
                    const SizedBox(width: 12),
                    ElevatedButton.icon(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: colorTipo,
                        foregroundColor: Colors.white,
                        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                      ),
                      onPressed: _isSaving ? null : _guardar,
                      icon: _isSaving
                          ? const SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                            )
                          : const Icon(Icons.save_outlined),
                      label: Text(_isSaving ? 'Guardando...' : 'Guardar Operación'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
