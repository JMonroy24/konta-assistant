import 'package:intl/intl.dart';
import 'package:uuid/uuid.dart';

enum TipoOperacion {
  venta,
  compra;

  String get displayName => this == TipoOperacion.venta ? 'Venta' : 'Compra';
  String get contraparteLabel => this == TipoOperacion.venta ? 'Cliente' : 'Proveedor';
}

/// Helper para sanitizar y parsear montos ingresados por el usuario
/// tolerando comas, puntos, espacios y símbolos de moneda.
class MontoParser {
  static double? parse(dynamic input) {
    if (input == null) return null;
    if (input is num) return input.toDouble();
    if (input is! String) return null;

    var cleaned = input.replaceAll(r'$', '').replaceAll(' ', '').trim();
    if (cleaned.isEmpty) return null;

    // Caso 1: Tiene comas y puntos (ej. 1,234.56 o 1.234,56)
    if (cleaned.contains(',') && cleaned.contains('.')) {
      if (cleaned.lastIndexOf('.') > cleaned.lastIndexOf(',')) {
        // Formato estadounidense: 1,234.56
        cleaned = cleaned.replaceAll(',', '');
      } else {
        // Formato europeo: 1.234,56
        cleaned = cleaned.replaceAll('.', '').replaceAll(',', '.');
      }
    } else if (cleaned.contains(',')) {
      // Caso 2: Solo contiene coma decimal (ej. 100,50)
      cleaned = cleaned.replaceAll(',', '.');
    }

    return double.tryParse(cleaned);
  }
}

/// Resultado de desglosar un importe que incluye IVA manteniendo
/// estrictamente los invariantes de Clean Architecture de Rust.
class DesgloseIvaResult {
  const DesgloseIvaResult({
    required this.base,
    required this.iva,
    required this.total,
    required this.ajusteCentavos,
  });

  final double base;
  final double iva;
  final double total;
  final double ajusteCentavos;

  bool get huboAjuste => ajusteCentavos.abs() > 0.001;
}

/// Modelo de Operación (Venta o Compra) conforme a la especificación
/// de Clean Architecture, esquemas JSON y AGENTS.md (moneda en string con 2 decimales).
class OperacionModel {
  OperacionModel({
    required this.id,
    this.rev,
    required this.tipo,
    required this.fecha,
    required this.descripcion,
    required this.base,
    required this.iva,
    required this.total,
    required this.contraparte,
    required this.creadoPor,
    this.origen = 'local',
    required this.creadoEn,
    this.isPendingSync = false,
  });

  final String id;
  final String? rev;
  final TipoOperacion tipo;
  final DateTime fecha;
  final String descripcion;
  final double base;
  final double iva;
  final double total;
  final String contraparte;
  final String creadoPor;
  final String origen;
  final DateTime creadoEn;
  final bool isPendingSync;

  static const double tasaIvaPorDefecto = 0.13;
  static const Uuid _uuid = Uuid();

  /// Crea una nueva operación con ID autogenerado idempotente y cálculo fiscal automático
  factory OperacionModel.crear({
    required TipoOperacion tipo,
    required DateTime fecha,
    required String descripcion,
    required double base,
    required String contraparte,
    required String creadoPor,
    double tasaIva = tasaIvaPorDefecto,
    String origen = 'local',
  }) {
    final baseNormalizada = redondear2Decimales(base);
    final ivaCalculado = calcularIva(baseNormalizada, tasa: tasaIva);
    final totalCalculado = redondear2Decimales(baseNormalizada + ivaCalculado);
    final dateStr = DateFormat('yyyy-MM-dd').format(fecha);
    final uniqueId = _uuid.v4();
    final prefix = tipo == TipoOperacion.venta ? 'venta' : 'compra';

    return OperacionModel(
      id: '$prefix:$dateStr:$uniqueId',
      tipo: tipo,
      fecha: fecha,
      descripcion: descripcion.trim(),
      base: baseNormalizada,
      iva: ivaCalculado,
      total: totalCalculado,
      contraparte: contraparte.trim(),
      creadoPor: creadoPor.trim(),
      origen: origen,
      creadoEn: DateTime.now().toUtc(),
      isPendingSync: true,
    );
  }

  /// Redondeo Half-Up exacto a 2 decimales equivalente a MidpointAwayFromZero en Rust
  static double redondear2Decimales(double valor) {
    if (valor.isNaN || valor.isInfinite) return 0.0;
    return (valor * 100).round() / 100.0;
  }

  /// Cálculo de IVA (13% por defecto en El Salvador)
  static double calcularIva(double baseImponible, {double tasa = tasaIvaPorDefecto}) {
    return redondear2Decimales(baseImponible * tasa);
  }

  /// Desglosa un total que ya incluye IVA garantizando que los invariantes
  /// de Rust (total == base + iva && iva == round(base * 0.13)) siempre se cumplan.
  static DesgloseIvaResult desglosarTotalConIva(
    double totalObjetivo, {
    double tasa = tasaIvaPorDefecto,
  }) {
    if (totalObjetivo <= 0) {
      return const DesgloseIvaResult(base: 0.0, iva: 0.0, total: 0.0, ajusteCentavos: 0.0);
    }

    double base = redondear2Decimales(totalObjetivo / (1.0 + tasa));
    double iva = calcularIva(base, tasa: tasa);
    double suma = redondear2Decimales(base + iva);

    // Si hay discrepancia de redondeo por la naturaleza de los enteros en centavos:
    if ((suma - totalObjetivo).abs() > 0.001) {
      final diff = totalObjetivo - suma;
      final baseAlt = redondear2Decimales(base + (diff > 0 ? 0.01 : -0.01));
      final ivaAlt = calcularIva(baseAlt, tasa: tasa);
      final sumaAlt = redondear2Decimales(baseAlt + ivaAlt);

      if ((sumaAlt - totalObjetivo).abs() < (suma - totalObjetivo).abs()) {
        base = baseAlt;
        iva = ivaAlt;
        suma = sumaAlt;
      }
    }

    return DesgloseIvaResult(
      base: base,
      iva: iva,
      total: suma,
      ajusteCentavos: redondear2Decimales(suma - totalObjetivo),
    );
  }

  /// Valida invariantes fiscales locales antes de emitir o encolar
  String? validar() {
    if (descripcion.trim().isEmpty) {
      return 'La descripción no puede estar vacía.';
    }
    if (contraparte.trim().isEmpty) {
      return 'El campo ${tipo.contraparteLabel} es obligatorio.';
    }
    if (base <= 0) {
      return 'La base imponible debe ser mayor a \$0.00.';
    }
    if (iva < 0) {
      return 'El IVA no puede ser negativo.';
    }
    final expectedIva = calcularIva(base);
    if ((iva - expectedIva).abs() > 0.009) {
      return 'El IVA ($iva) no corresponde a la tasa legal vigente ($expectedIva).';
    }
    final expectedTotal = redondear2Decimales(base + iva);
    if ((total - expectedTotal).abs() > 0.009) {
      return 'Incoherencia fiscal: el total ($total) no coincide con base ($base) + IVA ($iva).';
    }
    return null;
  }

  OperacionModel copyWith({
    String? id,
    String? rev,
    TipoOperacion? tipo,
    DateTime? fecha,
    String? descripcion,
    double? base,
    double? iva,
    double? total,
    String? contraparte,
    String? creadoPor,
    String? origen,
    DateTime? creadoEn,
    bool? isPendingSync,
  }) {
    return OperacionModel(
      id: id ?? this.id,
      rev: rev ?? this.rev,
      tipo: tipo ?? this.tipo,
      fecha: fecha ?? this.fecha,
      descripcion: descripcion ?? this.descripcion,
      base: base ?? this.base,
      iva: iva ?? this.iva,
      total: total ?? this.total,
      contraparte: contraparte ?? this.contraparte,
      creadoPor: creadoPor ?? this.creadoPor,
      origen: origen ?? this.origen,
      creadoEn: creadoEn ?? this.creadoEn,
      isPendingSync: isPendingSync ?? this.isPendingSync,
    );
  }

  /// Serialización JSON conforme a contratos.md y schemas/venta.json / compra.json
  /// con fechas en formato YYYY-MM-DD y números representados como cadenas decimales con 2 dígitos.
  Map<String, dynamic> toJson() {
    final map = <String, dynamic>{
      '_id': id,
      if (rev != null && rev!.isNotEmpty) '_rev': rev,
      'tipo': tipo == TipoOperacion.venta ? 'venta' : 'compra',
      'fecha': DateFormat('yyyy-MM-dd').format(fecha),
      'descripcion': descripcion.trim(),
      'base': base.toStringAsFixed(2),
      'iva': iva.toStringAsFixed(2),
      'total': total.toStringAsFixed(2),
      'contraparte': contraparte.trim(),
      'creado_por': creadoPor.trim(),
      'origen': origen,
      'creado_en': creadoEn.toUtc().toIso8601String(),
    };
    return map;
  }

  factory OperacionModel.fromJson(Map<String, dynamic> json, {bool isPending = false}) {
    final tipoStr = (json['tipo'] as String?)?.toLowerCase() ?? 'venta';
    final tipo = tipoStr == 'compra' ? TipoOperacion.compra : TipoOperacion.venta;

    DateTime fechaParsed;
    try {
      fechaParsed = json['fecha'] != null
          ? DateTime.parse(json['fecha'] as String)
          : DateTime.now();
    } catch (_) {
      fechaParsed = DateTime.now();
    }

    DateTime creadoEnParsed;
    try {
      creadoEnParsed = json['creado_en'] != null
          ? DateTime.parse(json['creado_en'] as String)
          : DateTime.now().toUtc();
    } catch (_) {
      creadoEnParsed = DateTime.now().toUtc();
    }

    final base = MontoParser.parse(json['base']) ?? 0.0;
    final iva = MontoParser.parse(json['iva']) ?? 0.0;
    final total = MontoParser.parse(json['total']) ?? 0.0;

    return OperacionModel(
      id: json['_id'] as String? ?? json['id'] as String? ?? '',
      rev: json['_rev'] as String?,
      tipo: tipo,
      fecha: fechaParsed,
      descripcion: (json['descripcion'] as String?)?.trim() ?? '',
      base: base,
      iva: iva,
      total: total,
      contraparte: (json['contraparte'] as String?)?.trim() ?? '',
      creadoPor: (json['creado_por'] as String?)?.trim() ??
          (json['creadoPor'] as String?)?.trim() ??
          '',
      origen: (json['origen'] as String?) ?? 'local',
      creadoEn: creadoEnParsed,
      isPendingSync: isPending,
    );
  }
}
