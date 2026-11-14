import 'package:flutter_test/flutter_test.dart';
import 'package:konta_assistant/features/ventas_compras/models/operacion_model.dart';

void main() {
  group('MontoParser - Casos de Borde y Sanitización de Entrada', () {
    test('Parsea montos con comas, puntos y símbolos correctamente', () {
      expect(MontoParser.parse(100), 100.0);
      expect(MontoParser.parse('100.50'), 100.50);
      expect(MontoParser.parse('100,50'), 100.50); // Coma como decimal latino
      expect(MontoParser.parse('\$1,234.56'), 1234.56); // Símbolo $ y coma de miles
      expect(MontoParser.parse('  250.75  '), 250.75); // Espacios en blanco
      expect(MontoParser.parse('1.234,56'), 1234.56); // Formato europeo
    });

    test('Retorna null ante entradas vacías o inválidas', () {
      expect(MontoParser.parse(''), isNull);
      expect(MontoParser.parse('abc'), isNull);
      expect(MontoParser.parse(null), isNull);
      expect(MontoParser.parse('\$  '), isNull);
    });
  });

  group('DesgloseIvaResult - Casos de Borde de Redondeo (Monto incluye IVA)', () {
    test('Garantiza invariantes fiscales estrictos incluso en huecos de redondeo', () {
      // Casos conflictivos conocidos matemáticamente donde (Total / 1.13) genera saltos:
      final casosSensibles = [1.00, 2.30, 3.00, 4.30, 5.00, 10.00, 100.00];

      for (final totalInput in casosSensibles) {
        final desglose = OperacionModel.desglosarTotalConIva(totalInput);

        // Invariante 1: total debe ser idéntico a base + iva
        final suma = OperacionModel.redondear2Decimales(desglose.base + desglose.iva);
        expect(desglose.total, suma, reason: 'Total no coincide con base + IVA para $totalInput');

        // Invariante 2: IVA debe ser exactamente base * 0.13 redondeado
        final ivaEsperado = OperacionModel.calcularIva(desglose.base);
        expect(desglose.iva, ivaEsperado, reason: 'IVA no cumple tasa del 13% para base ${desglose.base}');

        // Invariante 3: No debe diferir del valor original en más de 1 centavo
        expect((desglose.total - totalInput).abs() <= 0.015, isTrue,
            reason: 'Diferencia excesiva con el monto deseado');
      }
    });
  });

  group('OperacionModel - Cálculos Fiscales y Redondeo', () {
    test('Calcula IVA general al 13% con redondeo Half-Up exacto', () {
      expect(OperacionModel.calcularIva(100.0), 13.00);
      expect(OperacionModel.calcularIva(10.55), 1.37);
      expect(OperacionModel.calcularIva(10.58), 1.38);
      expect(OperacionModel.calcularIva(33.33), 4.33);
      expect(OperacionModel.calcularIva(70.05), 9.11);
    });

    test('OperacionModel.crear genera IDs con formato válido y fecha YYYY-MM-DD estricta', () {
      final fecha = DateTime(2026, 11, 5); // Día de 1 dígito
      final venta = OperacionModel.crear(
        tipo: TipoOperacion.venta,
        fecha: fecha,
        descripcion: 'Venta de prueba',
        base: 200.0,
        contraparte: 'Cliente ABC',
        creadoPor: 'admin',
      );

      // Verificación de padding de ceros obligatorio para CouchDB Mango Queries
      expect(venta.id.startsWith('venta:2026-11-05:'), isTrue);
      expect(venta.toJson()['fecha'], '2026-11-05');
      expect(venta.base, 200.0);
      expect(venta.iva, 26.0);
      expect(venta.total, 226.0);
      expect(venta.validar(), isNull);
    });

    test('Validación detecta campos vacíos, base negativa o inconsistencias', () {
      final fecha = DateTime(2026, 11, 14);

      final opBaseCero = OperacionModel(
        id: 'op1',
        tipo: TipoOperacion.venta,
        fecha: fecha,
        descripcion: 'Venta',
        base: 0.0,
        iva: 0.0,
        total: 0.0,
        contraparte: 'Cliente',
        creadoPor: 'user',
        creadoEn: DateTime.now(),
      );
      expect(opBaseCero.validar(), contains('mayor a \$0.00'));

      final opIvaIncoherente = OperacionModel(
        id: 'op2',
        tipo: TipoOperacion.venta,
        fecha: fecha,
        descripcion: 'Venta',
        base: 100.0,
        iva: 10.0, // Error: 100 * 0.13 = 13.0 != 10.0
        total: 110.0,
        contraparte: 'Cliente',
        creadoPor: 'user',
        creadoEn: DateTime.now(),
      );
      expect(opIvaIncoherente.validar(), contains('no corresponde'));
    });
  });
}
