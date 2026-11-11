import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

/// Cliente HTTP compartido por las funciones de la aplicación.
///
/// La URL puede configurarse al ejecutar Flutter mediante:
/// --dart-define=API_BASE_URL=http://<host>:8080
class ApiClient {
  ApiClient({String? baseUrl, http.Client? client})
      : baseUrl = _normalizeBaseUrl(baseUrl ?? defaultBaseUrl),
        _client = client ?? http.Client();

  final String baseUrl;
  final http.Client _client;

  static const String _configuredBaseUrl =
      String.fromEnvironment('API_BASE_URL');

  static String get defaultBaseUrl {
    if (_configuredBaseUrl.isNotEmpty) return _configuredBaseUrl;

    // 10.0.2.2 permite que el emulador Android alcance el localhost del host.
    if (!kIsWeb && defaultTargetPlatform == TargetPlatform.android) {
      return 'http://10.0.2.2:8080';
    }

    return 'http://localhost:8080';
  }

  static String _normalizeBaseUrl(String value) {
    final normalized = value.trim();
    if (normalized.isEmpty) {
      throw ArgumentError.value(value, 'baseUrl', 'No puede estar vacía.');
    }
    return normalized.endsWith('/')
        ? normalized.substring(0, normalized.length - 1)
        : normalized;
  }

  Map<String, String> _headers({String? authToken}) => {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
        if (authToken != null && authToken.isNotEmpty)
          'Authorization': 'Bearer $authToken',
      };

  Future<http.Response> get(
    String endpoint, {
    String? authToken,
  }) {
    return _client
        .get(
          _uri(endpoint),
          headers: _headers(authToken: authToken),
        )
        .timeout(const Duration(seconds: 15));
  }

  Future<http.Response> post(
    String endpoint,
    Map<String, dynamic> body, {
    String? authToken,
  }) {
    return _client
        .post(
          _uri(endpoint),
          headers: _headers(authToken: authToken),
          body: jsonEncode(body),
        )
        .timeout(const Duration(seconds: 15));
  }

  Uri _uri(String endpoint) {
    final path = endpoint.startsWith('/') ? endpoint : '/$endpoint';
    return Uri.parse('$baseUrl$path');
  }

  void close() => _client.close();
}
