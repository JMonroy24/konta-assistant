import 'dart:convert';
import 'package:http/http.dart' as http;

class ApiClient {
  final String baseUrl;
  String? authToken;

  ApiClient({required this.baseUrl});

  Map<String, String> _headers() {
    return {
      'Content-Type': 'application/json',
      if (authToken != null) 'Authorization': 'Bearer $authToken',
    };
  }

  Future<http.Response> get(String endpoint) async {
    final uri = Uri.parse('$baseUrl$endpoint');
    return await http.get(uri, headers: _headers());
  }

  Future<http.Response> post(String endpoint, Map<String, dynamic> body) async {
    final uri = Uri.parse('$baseUrl$endpoint');
    return await http.post(uri, headers: _headers(), body: jsonEncode(body));
  }
}
