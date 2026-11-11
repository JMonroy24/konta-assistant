import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'http_client.dart';

class AuthSession {
  const AuthSession({
    required this.token,
    required this.username,
    required this.role,
    required this.businessId,
  });

  final String token;
  final String username;
  final String role;
  final String businessId;

  factory AuthSession.fromJson(Map<String, dynamic> json) {
    final token = json['token'];
    final username = json['username'];
    final role = json['rol'] ?? json['role'] ?? 'lectura';
    final businessId = json['negocio_id'] ?? json['business_id'] ?? '';

    if (token is! String || username is! String ||
        role is! String || businessId is! String) {
      throw const FormatException(
        'La respuesta de autenticación contiene campos inválidos.',
      );
    }

    return AuthSession(
      token: token,
      username: username,
      role: role,
      businessId: businessId,
    );
  }

  Map<String, dynamic> toJson() => {
        'token': token,
        'username': username,
        'rol': role,
        'negocio_id': businessId,
      };
}

class AuthException implements Exception {
  const AuthException(this.message);

  final String message;

  @override
  String toString() => message;
}

class AuthService {
  AuthService({
    ApiClient? apiClient,
    FlutterSecureStorage? secureStorage,
  })  : _apiClient = apiClient ?? ApiClient(),
        _secureStorage = secureStorage ?? const FlutterSecureStorage();

  static const String _sessionKey = 'konta_auth_session';

  final ApiClient _apiClient;
  final FlutterSecureStorage _secureStorage;

  Future<AuthSession> login({
    required String username,
    required String password,
  }) async {
    try {
      final response = await _apiClient.post(
        '/auth/login',
        {
          'username': username.trim(),
          'password': password,
        },
      );

      final Map<String, dynamic> payload = _decodeResponse(response.body);

      if (response.statusCode != 200) {
        throw AuthException(
          _errorMessage(payload) ??
              'No fue posible iniciar sesión. Verifica tus credenciales.',
        );
      }

      final session = AuthSession.fromJson(payload);
      if (session.token.trim().isEmpty || session.username.trim().isEmpty) {
        throw const AuthException(
          'El servidor devolvió una respuesta de autenticación incompleta.',
        );
      }

      await _secureStorage.write(
        key: _sessionKey,
        value: jsonEncode(session.toJson()),
      );

      return session;
    } on AuthException {
      rethrow;
    } on http.ClientException {
      throw const AuthException(
        'No se pudo conectar con el servidor. Comprueba tu conexión y la URL de la API.',
      );
    } on TimeoutException {
      throw const AuthException(
        'El servidor tardó demasiado en responder. Inténtalo nuevamente.',
      );
    } on FormatException {
      throw const AuthException(
        'El servidor devolvió una respuesta que no tiene un formato válido.',
      );
    } on Exception {
      throw const AuthException(
        'Ocurrió un error al iniciar sesión. Inténtalo nuevamente.',
      );
    }
  }

  Future<AuthSession?> restoreSession() async {
    final storedSession = await _secureStorage.read(key: _sessionKey);
    if (storedSession == null || storedSession.isEmpty) return null;

    try {
      final decoded = jsonDecode(storedSession) as Map<String, dynamic>;
      final session = AuthSession.fromJson(decoded);
      if (session.token.isEmpty) return null;
      return session;
    } on FormatException {
      await logout();
      return null;
    } on TypeError {
      await logout();
      return null;
    }
  }

  Future<void> logout() => _secureStorage.delete(key: _sessionKey);

  Map<String, dynamic> _decodeResponse(String body) {
    if (body.trim().isEmpty) return <String, dynamic>{};
    final decoded = jsonDecode(body);
    if (decoded is Map<String, dynamic>) return decoded;
    throw const FormatException('La respuesta no es un objeto JSON.');
  }

  String? _errorMessage(Map<String, dynamic> payload) {
    final value = payload['error'] ?? payload['mensaje'] ?? payload['message'];
    if (value is String && value.trim().isNotEmpty) return value.trim();
    return null;
  }
}
