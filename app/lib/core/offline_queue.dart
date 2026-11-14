import 'dart:collection';
import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';

class QueueItem {
  QueueItem({
    required this.id,
    required this.endpoint,
    required this.payload,
    required this.timestamp,
    this.retryCount = 0,
    this.lastError,
  });

  final String id;
  final String endpoint;
  final Map<String, dynamic> payload;
  final DateTime timestamp;
  int retryCount;
  String? lastError;

  Map<String, dynamic> toJson() => {
        'id': id,
        'endpoint': endpoint,
        'payload': payload,
        'timestamp': timestamp.toUtc().toIso8601String(),
        'retry_count': retryCount,
        if (lastError != null) 'last_error': lastError,
      };

  factory QueueItem.fromJson(Map<String, dynamic> json) => QueueItem(
        id: json['id'] as String? ?? '',
        endpoint: json['endpoint'] as String? ?? '',
        payload: Map<String, dynamic>.from(json['payload'] as Map? ?? {}),
        timestamp: json['timestamp'] != null
            ? DateTime.tryParse(json['timestamp'] as String) ?? DateTime.now()
            : DateTime.now(),
        retryCount: json['retry_count'] as int? ?? 0,
        lastError: json['last_error'] as String?,
      );
}

class OfflineQueue {
  OfflineQueue({SharedPreferences? prefs}) : _prefs = prefs;

  final Queue<QueueItem> _queue = Queue<QueueItem>();
  final List<QueueItem> _deadLetterQueue = [];
  SharedPreferences? _prefs;

  static const String _storageKey = 'konta_offline_queue_items';
  static const String _deadLetterKey = 'konta_dead_letter_items';
  static const int maxRetries = 5;

  Future<void> init() async {
    _prefs ??= await SharedPreferences.getInstance();
    final rawList = _prefs?.getStringList(_storageKey);
    if (rawList != null) {
      _queue.clear();
      for (final raw in rawList) {
        try {
          final map = jsonDecode(raw) as Map<String, dynamic>;
          _queue.addLast(QueueItem.fromJson(map));
        } catch (_) {
          // Si una entrada está corrupta, se omite para no quebrar la app
        }
      }
    }

    final rawDead = _prefs?.getStringList(_deadLetterKey);
    if (rawDead != null) {
      _deadLetterQueue.clear();
      for (final raw in rawDead) {
        try {
          final map = jsonDecode(raw) as Map<String, dynamic>;
          _deadLetterQueue.add(QueueItem.fromJson(map));
        } catch (_) {}
      }
    }
  }

  Future<void> _persist() async {
    if (_prefs == null) {
      try {
        _prefs = await SharedPreferences.getInstance();
      } catch (_) {}
    }
    final rawList = _queue.map((item) => jsonEncode(item.toJson())).toList();
    await _prefs?.setStringList(_storageKey, rawList);

    final rawDead = _deadLetterQueue.map((item) => jsonEncode(item.toJson())).toList();
    await _prefs?.setStringList(_deadLetterKey, rawDead);
  }

  void enqueue(QueueItem item) {
    if (item.id.trim().isEmpty) return;
    // Evitar duplicados por id si ya está en cola
    _queue.removeWhere((existing) => existing.id == item.id);
    _deadLetterQueue.removeWhere((existing) => existing.id == item.id);
    _queue.addLast(item);
    _persist();
  }

  QueueItem? dequeue() {
    if (_queue.isEmpty) return null;
    final item = _queue.removeFirst();
    _persist();
    return item;
  }

  void registrarFallo(String id, String error, {bool esPermanente = false}) {
    QueueItem? target;
    for (final item in _queue) {
      if (item.id == id) {
        target = item;
        break;
      }
    }

    if (target != null) {
      target.retryCount++;
      target.lastError = error;

      if (esPermanente || target.retryCount >= maxRetries) {
        // Mover a cola de errores permanentes (dead letter) para no bloquear la cola principal
        _queue.removeWhere((i) => i.id == id);
        _deadLetterQueue.add(target);
      }
      _persist();
    }
  }

  int get pendingCount => _queue.length;

  int get deadLetterCount => _deadLetterQueue.length;

  List<QueueItem> get allPending => _queue.toList();

  List<QueueItem> get allDeadLetters => List.unmodifiable(_deadLetterQueue);

  void removeById(String id) {
    _queue.removeWhere((item) => item.id == id);
    _deadLetterQueue.removeWhere((item) => item.id == id);
    _persist();
  }

  void clear() {
    _queue.clear();
    _deadLetterQueue.clear();
    _persist();
  }
}
