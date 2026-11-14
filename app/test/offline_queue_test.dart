import 'package:flutter_test/flutter_test.dart';
import 'package:konta_assistant/core/offline_queue.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  group('OfflineQueue Tests - Casos de Borde y Resiliencia', () {
    setUp(() {
      SharedPreferences.setMockInitialValues({});
    });

    test('Encola y desencola elementos en orden FIFO', () {
      final queue = OfflineQueue();

      final item1 = QueueItem(
        id: 'op-1',
        endpoint: '/ventas',
        payload: {'base': '100.00'},
        timestamp: DateTime.now(),
      );
      final item2 = QueueItem(
        id: 'op-2',
        endpoint: '/compras',
        payload: {'base': '50.00'},
        timestamp: DateTime.now(),
      );

      queue.enqueue(item1);
      queue.enqueue(item2);

      expect(queue.pendingCount, 2);

      final dequeued1 = queue.dequeue();
      expect(dequeued1?.id, 'op-1');
      expect(queue.pendingCount, 1);

      final dequeued2 = queue.dequeue();
      expect(dequeued2?.id, 'op-2');
      expect(queue.pendingCount, 0);

      expect(queue.dequeue(), isNull);
    });

    test('Aísla fallos permanentes en cola de errores (dead-letter) sin bloquear la principal', () {
      final queue = OfflineQueue();

      final itemMalformado = QueueItem(
        id: 'bad-item',
        endpoint: '/ventas',
        payload: {'invalid': true},
        timestamp: DateTime.now(),
      );
      final itemValido = QueueItem(
        id: 'good-item',
        endpoint: '/ventas',
        payload: {'base': '100.00'},
        timestamp: DateTime.now(),
      );

      queue.enqueue(itemMalformado);
      queue.enqueue(itemValido);

      expect(queue.pendingCount, 2);

      // Simular rechazo 400 permanente
      queue.registrarFallo('bad-item', 'HTTP 400 Bad Request', esPermanente: true);

      // La cola principal se descongestiona
      expect(queue.pendingCount, 1);
      expect(queue.allPending.first.id, 'good-item');

      // El error se conserva en dead-letter para auditoría
      expect(queue.deadLetterCount, 1);
      expect(queue.allDeadLetters.first.id, 'bad-item');
      expect(queue.allDeadLetters.first.lastError, contains('400'));
    });

    test('Maneja entradas corruptas en SharedPreferences sin lanzar excepciones', () async {
      SharedPreferences.setMockInitialValues({
        'konta_offline_queue_items': [
          '{"id": "op-ok", "endpoint": "/ventas", "payload": {}, "timestamp": "2026-11-14T10:00:00Z"}',
          '{corrupted json string...}',
        ],
      });

      final queue = OfflineQueue();
      await queue.init();

      // Debe haber cargado el elemento válido omitiendo el corrupto
      expect(queue.pendingCount, 1);
      expect(queue.allPending.first.id, 'op-ok');
    });
  });
}
