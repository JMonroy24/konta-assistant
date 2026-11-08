import 'offline_queue.dart';
import 'http_client.dart';

enum SyncState { idle, syncing, offline, error }

class SyncService {
  final ApiClient apiClient;
  final OfflineQueue offlineQueue;
  SyncState state = SyncState.idle;

  SyncService({required this.apiClient, required this.offlineQueue});

  Future<void> syncPending() async {
    if (offlineQueue.pendingCount == 0) return;

    state = SyncState.syncing;
    while (offlineQueue.pendingCount > 0) {
      final item = offlineQueue.dequeue();
      if (item == null) break;

      try {
        final res = await apiClient.post(item.endpoint, item.payload);
        if (res.statusCode >= 400) {
          // Re-encolar si hubo error de conexión
          offlineQueue.enqueue(item);
          state = SyncState.error;
          break;
        }
      } catch (_) {
        offlineQueue.enqueue(item);
        state = SyncState.offline;
        break;
      }
    }

    if (offlineQueue.pendingCount == 0) {
      state = SyncState.idle;
    }
  }
}
