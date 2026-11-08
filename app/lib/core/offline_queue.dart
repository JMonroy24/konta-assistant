import 'dart:collection';

class QueueItem {
  final String id;
  final String endpoint;
  final Map<String, dynamic> payload;
  final DateTime timestamp;

  QueueItem({
    required this.id,
    required this.endpoint,
    required this.payload,
    required this.timestamp,
  });
}

class OfflineQueue {
  final Queue<QueueItem> _queue = Queue<QueueItem>();

  void enqueue(QueueItem item) {
    _queue.addLast(item);
  }

  QueueItem? dequeue() {
    if (_queue.isEmpty) return null;
    return _queue.removeFirst();
  }

  int get pendingCount => _queue.length;

  List<QueueItem> get allPending => _queue.toList();
}
