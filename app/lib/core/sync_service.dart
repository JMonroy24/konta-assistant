import 'http_client.dart';
import 'offline_queue.dart';

enum SyncState { idle, syncing, offline, error, success, unauthorized }

class SyncResult {
  SyncResult({
    required this.sincronizados,
    required this.fallidos,
    required this.mensaje,
    required this.estadoFinal,
  });

  final int sincronizados;
  final int fallidos;
  final String mensaje;
  final SyncState estadoFinal;
}

class SyncService {
  SyncService({
    ApiClient? apiClient,
    OfflineQueue? offlineQueue,
  })  : apiClient = apiClient ?? ApiClient(),
        offlineQueue = offlineQueue ?? OfflineQueue();

  final ApiClient apiClient;
  final OfflineQueue offlineQueue;
  SyncState state = SyncState.idle;

  Future<SyncResult> syncPending({String? authToken}) async {
    final totalPendientes = offlineQueue.pendingCount;
    if (totalPendientes == 0) {
      state = SyncState.idle;
      return SyncResult(
        sincronizados: 0,
        fallidos: 0,
        mensaje: 'No hay operaciones pendientes de sincronización.',
        estadoFinal: SyncState.idle,
      );
    }

    state = SyncState.syncing;
    int procesados = 0;
    int erroresPermanentes = 0;

    // Trabajamos sobre una copia de los pendientes actuales
    final List<QueueItem> items = offlineQueue.allPending;

    for (final item in items) {
      try {
        final res = await apiClient.post(
          item.endpoint,
          item.payload,
          authToken: authToken,
        );

        if (res.statusCode == 200 || res.statusCode == 201) {
          // Éxito: aplicación en el servidor
          offlineQueue.removeById(item.id);
          procesados++;
        } else if (res.statusCode == 409) {
          // Conflicto de CouchDB: el documento ya existe en el servidor (idempotencia cumplida)
          offlineQueue.removeById(item.id);
          procesados++;
        } else if (res.statusCode == 401 || res.statusCode == 403) {
          // Error de autenticación: token inválido o expirado
          state = SyncState.unauthorized;
          return SyncResult(
            sincronizados: procesados,
            fallidos: 1,
            mensaje: 'Sesión expirada o no autorizada. Inicia sesión nuevamente para sincronizar.',
            estadoFinal: SyncState.unauthorized,
          );
        } else if (res.statusCode == 400 || res.statusCode == 422) {
          // Error permanente en la carga útil: aislar a la cola de fallos para no bloquear la cola principal
          offlineQueue.registrarFallo(
            item.id,
            'Error ${res.statusCode}: ${res.body}',
            esPermanente: true,
          );
          erroresPermanentes++;
        } else {
          // Error transitorio de servidor (5xx): registrar intento y pausar para no saturar
          offlineQueue.registrarFallo(
            item.id,
            'Error transitorio del servidor: ${res.statusCode}',
            esPermanente: false,
          );
          state = SyncState.error;
          break;
        }
      } catch (e) {
        // Excepción de red o timeout
        offlineQueue.registrarFallo(
          item.id,
          'Error de conectividad: $e',
          esPermanente: false,
        );
        state = SyncState.offline;
        break;
      }
    }

    if (offlineQueue.pendingCount == 0) {
      state = SyncState.success;
      return SyncResult(
        sincronizados: procesados,
        fallidos: erroresPermanentes,
        mensaje: erroresPermanentes > 0
            ? 'Se sincronizaron $procesados operaciones. $erroresPermanentes fueron rechazadas por formato.'
            : 'Todas las operaciones pendientes fueron sincronizadas exitosamente.',
        estadoFinal: SyncState.success,
      );
    } else {
      return SyncResult(
        sincronizados: procesados,
        fallidos: offlineQueue.pendingCount,
        mensaje: state == SyncState.offline
            ? 'Sin conexión al servidor. Se sincronizaron $procesados operaciones. Restan ${offlineQueue.pendingCount}.'
            : 'Se sincronizaron $procesados operaciones. Quedan ${offlineQueue.pendingCount} pendientes.',
        estadoFinal: state,
      );
    }
  }
}
