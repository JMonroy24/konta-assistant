import 'package:flutter_test/flutter_test.dart';
import 'package:konta_assistant/core/offline_queue.dart';
import 'package:konta_assistant/features/ventas_compras/models/operacion_model.dart';
import 'package:konta_assistant/features/ventas_compras/services/operaciones_service.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  group('OperacionesService - Cálculos y Resumen', () {
    setUp(() {
      SharedPreferences.setMockInitialValues({});
    });

    test('calcularResumen calcula correctamente totales de ventas, compras y saldos de IVA', () {
      final service = OperacionesService();
      final fecha = DateTime(2026, 11, 14);

      final operaciones = [
        OperacionModel.crear(
          tipo: TipoOperacion.venta,
          fecha: fecha,
          descripcion: 'Venta 1',
          base: 100.0, // IVA: 13.00, Total: 113.00
          contraparte: 'Cliente A',
          creadoPor: 'benito',
        ),
        OperacionModel.crear(
          tipo: TipoOperacion.venta,
          fecha: fecha,
          descripcion: 'Venta 2',
          base: 50.0, // IVA: 6.50, Total: 56.50
          contraparte: 'Cliente B',
          creadoPor: 'benito',
        ),
        OperacionModel.crear(
          tipo: TipoOperacion.compra,
          fecha: fecha,
          descripcion: 'Compra Insumos',
          base: 80.0, // IVA: 10.40, Total: 90.40
          contraparte: 'Proveedor C',
          creadoPor: 'benito',
        ),
      ];

      final resumen = service.calcularResumen(operaciones);

      expect(resumen.cantidadOperaciones, 3);
      expect(resumen.totalVentas, 150.0);
      expect(resumen.totalCompras, 80.0);
      expect(resumen.debitoIva, 19.50); // 13.00 + 6.50
      expect(resumen.creditoIva, 10.40);
      expect(resumen.ivaNeto, 9.10); // 19.50 - 10.40
    });

    test('registrarOperacion con forceOffline guarda en cola offline y caché local', () async {
      final queue = OfflineQueue();
      await queue.init();
      final service = OperacionesService(offlineQueue: queue);

      final op = OperacionModel.crear(
        tipo: TipoOperacion.compra,
        fecha: DateTime(2026, 11, 14),
        descripcion: 'Compra de material de oficina',
        base: 45.0,
        contraparte: 'Librería Central',
        creadoPor: 'benito',
      );

      final guardada = await service.registrarOperacion(op, forceOffline: true);

      expect(guardada.isPendingSync, isTrue);
      expect(queue.pendingCount, 1);
      expect(queue.allPending.first.endpoint, '/compras');
      expect(queue.allPending.first.payload['_id'], op.id);
    });
  });
}

