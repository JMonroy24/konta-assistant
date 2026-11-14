import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:intl/intl.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../../core/http_client.dart';
import '../../../core/offline_queue.dart';
import '../models/operacion_model.dart';

class ResumenOperaciones {
  const ResumenOperaciones({
    required this.totalVentas,
    required this.totalCompras,
    required this.debitoIva,
    required this.creditoIva,
    required this.ivaNeto,
    required this.cantidadOperaciones,
  });

  final double totalVentas;
  final double totalCompras;
  final double debitoIva;
  final double creditoIva;
  final double ivaNeto;
  final int cantidadOperaciones;
}

class OperacionesService {
  OperacionesService({
    ApiClient? apiClient,
    OfflineQueue? offlineQueue,
    SharedPreferences? prefs,
  })  : _apiClient = apiClient ?? ApiClient(),
        _offlineQueue = offlineQueue ?? OfflineQueue(),
        _prefs = prefs;

  final ApiClient _apiClient;
  final OfflineQueue _offlineQueue;
  SharedPreferences? _prefs;

  static const String _cacheKeyPrefix = 'konta_local_operaciones_cache_';

  Future<void> _ensurePrefs() async {
    _prefs ??= await SharedPreferences.getInstance();
  }

  /// Registra una operación. Si hay red la envía al backend (/ventas o /compras).
  /// Si ocurre fallo de red o está offline, se encola localmente de forma idempotente.
  Future<OperacionModel> registrarOperacion(
    OperacionModel operacion, {
    String? authToken,
    bool forceOffline = false,
  }) async {
    final validationError = operacion.validar();
    if (validationError != null) {
      throw ArgumentError(validationError);
    }

    final endpoint = operacion.tipo == TipoOperacion.venta ? '/ventas' : '/compras';
    final payload = operacion.toJson();

    if (!forceOffline) {
      try {
        final response = await _apiClient.post(
          endpoint,
          payload,
          authToken: authToken,
        );

        if (response.statusCode == 200 || response.statusCode == 201) {
          final savedOp = operacion.copyWith(
            isPendingSync: false,
            origen: 'remoto',
          );
          await _guardarEnCacheLocal(savedOp);
          return savedOp;
        } else if (response.statusCode == 409) {
          // Ya existía en el servidor: tratar como sincronizada
          final savedOp = operacion.copyWith(
            isPendingSync: false,
            origen: 'remoto',
          );
          await _guardarEnCacheLocal(savedOp);
          return savedOp;
        }
      } catch (e) {
        debugPrint('Fallo al enviar operación al backend: $e. Encolando en cola offline...');
      }
    }

    // Guardado local y encolado para sincronización posterior
    final pendingOp = operacion.copyWith(isPendingSync: true, origen: 'local');
    _offlineQueue.enqueue(QueueItem(
      id: pendingOp.id,
      endpoint: endpoint,
      payload: payload,
      timestamp: DateTime.now().toUtc(),
    ));

    await _guardarEnCacheLocal(pendingOp);
    return pendingOp;
  }

  /// Consulta operaciones para un período dado (AAAA-MM), fusionando
  /// los datos del backend con las operaciones locales aún pendientes de sincronización.
  Future<List<OperacionModel>> listarOperaciones({
    required String periodoMes, // '2026-11'
    String? authToken,
  }) async {
    final List<OperacionModel> remotas = [];
    final List<OperacionModel> locales = await _obtenerDeCacheLocal(periodoMes);

    try {
      final response = await _apiClient.get(
        '/operaciones?mes=$periodoMes',
        authToken: authToken,
      );

      if (response.statusCode == 200) {
        final decoded = jsonDecode(response.body);
        final list = decoded is Map && decoded['operaciones'] is List
            ? decoded['operaciones'] as List
            : (decoded is List ? decoded : []);

        for (final item in list) {
          if (item is Map<String, dynamic>) {
            try {
              remotas.add(OperacionModel.fromJson(item, isPending: false));
            } catch (_) {}
          }
        }
      }
    } catch (e) {
      debugPrint('No fue posible consultar operaciones remotas: $e. Utilizando caché local.');
    }

    // Mapa unificado indexado por _id
    final Map<String, OperacionModel> unificadas = {};

    // 1. Añadir operaciones remotas oficiales
    for (final op in remotas) {
      if (op.id.isNotEmpty) {
        unificadas[op.id] = op;
      }
    }

    // 2. Añadir pendientes de la cola offline que correspondan al período
    final pendientesEnCola = _offlineQueue.allPending;
    for (final item in pendientesEnCola) {
      try {
        final op = OperacionModel.fromJson(item.payload, isPending: true);
        final mesOp = DateFormat('yyyy-MM').format(op.fecha);
        if (mesOp == periodoMes && op.id.isNotEmpty) {
          unificadas[op.id] = op;
        }
      } catch (_) {}
    }

    // 3. Añadir de la caché local las no presentes
    for (final op in locales) {
      if (op.id.isNotEmpty && !unificadas.containsKey(op.id)) {
        unificadas[op.id] = op;
      }
    }

    final listaResultado = unificadas.values.toList();
    // Ordenar por fecha descendente
    listaResultado.sort((a, b) => b.fecha.compareTo(a.fecha));
    return listaResultado;
  }

  /// Calcula el resumen fiscal de una lista de operaciones con redondeo Half-Up exacto
  ResumenOperaciones calcularResumen(List<OperacionModel> operaciones) {
    double totalVentas = 0.0;
    double totalCompras = 0.0;
    double debitoIva = 0.0;
    double creditoIva = 0.0;

    for (final op in operaciones) {
      if (op.tipo == TipoOperacion.venta) {
        totalVentas += op.base;
        debitoIva += op.iva;
      } else {
        totalCompras += op.base;
        creditoIva += op.iva;
      }
    }

    totalVentas = OperacionModel.redondear2Decimales(totalVentas);
    totalCompras = OperacionModel.redondear2Decimales(totalCompras);
    debitoIva = OperacionModel.redondear2Decimales(debitoIva);
    creditoIva = OperacionModel.redondear2Decimales(creditoIva);
    final ivaNeto = OperacionModel.redondear2Decimales(debitoIva - creditoIva);

    return ResumenOperaciones(
      totalVentas: totalVentas,
      totalCompras: totalCompras,
      debitoIva: debitoIva,
      creditoIva: creditoIva,
      ivaNeto: ivaNeto,
      cantidadOperaciones: operaciones.length,
    );
  }

  Future<void> _guardarEnCacheLocal(OperacionModel op) async {
    try {
      await _ensurePrefs();
      final mes = DateFormat('yyyy-MM').format(op.fecha);
      final key = '$_cacheKeyPrefix$mes';
      final existingRaw = _prefs?.getStringList(key) ?? [];

      final list = <Map<String, dynamic>>[];
      for (final e in existingRaw) {
        try {
          list.add(jsonDecode(e) as Map<String, dynamic>);
        } catch (_) {}
      }

      list.removeWhere((item) => (item['_id'] ?? item['id']) == op.id);
      list.add(op.toJson()..['isPendingSync'] = op.isPendingSync);

      final updatedRaw = list.map((e) => jsonEncode(e)).toList();
      await _prefs?.setStringList(key, updatedRaw);
    } catch (e) {
      debugPrint('Error guardando en caché local: $e');
    }
  }

  Future<List<OperacionModel>> _obtenerDeCacheLocal(String periodoMes) async {
    try {
      await _ensurePrefs();
      final key = '$_cacheKeyPrefix$periodoMes';
      final existingRaw = _prefs?.getStringList(key) ?? [];
      final List<OperacionModel> list = [];

      for (final raw in existingRaw) {
        try {
          final map = jsonDecode(raw) as Map<String, dynamic>;
          final isPending = map['isPendingSync'] == true;
          list.add(OperacionModel.fromJson(map, isPending: isPending));
        } catch (_) {}
      }
      return list;
    } catch (_) {
      return [];
    }
  }
}
