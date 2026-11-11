import 'package:flutter/material.dart';

import 'core/auth_service.dart';
import 'core/theme.dart';
import 'features/auth/login_screen.dart';
import 'features/dashboard/dashboard_screen.dart';
import 'features/ventas_compras/ventas_compras_screen.dart';
import 'features/facturacion/facturacion_screen.dart';
import 'features/declaraciones/declaraciones_screen.dart';
import 'features/recordatorios/recordatorios_screen.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const KontaApp());
}

class KontaApp extends StatefulWidget {
  const KontaApp({super.key});

  @override
  State<KontaApp> createState() => _KontaAppState();
}

class _KontaAppState extends State<KontaApp> {
  final AuthService _authService = AuthService();
  AuthSession? _session;
  bool _isRestoringSession = true;

  @override
  void initState() {
    super.initState();
    _restoreSession();
  }

  Future<void> _restoreSession() async {
    try {
      final session = await _authService.restoreSession();
      if (!mounted) return;
      setState(() {
        _session = session;
        _isRestoringSession = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _session = null;
        _isRestoringSession = false;
      });
    }
  }

  void _handleAuthenticated(AuthSession session) {
    setState(() => _session = session);
  }

  Future<void> _handleLogout() async {
    await _authService.logout();
    if (!mounted) return;
    setState(() => _session = null);
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Konta Assistant',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.lightTheme,
      home: _isRestoringSession
          ? const _LoadingScreen()
          : _session == null
              ? LoginScreen(onAuthenticated: _handleAuthenticated)
              : MainLayoutScreen(
                  session: _session!,
                  onLogout: _handleLogout,
                ),
    );
  }
}

class _LoadingScreen extends StatelessWidget {
  const _LoadingScreen();

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(child: CircularProgressIndicator()),
    );
  }
}

class MainLayoutScreen extends StatefulWidget {
  const MainLayoutScreen({
    required this.session,
    required this.onLogout,
    super.key,
  });

  final AuthSession session;
  final VoidCallback onLogout;

  @override
  State<MainLayoutScreen> createState() => _MainLayoutScreenState();
}

class _MainLayoutScreenState extends State<MainLayoutScreen> {
  int _currentIndex = 0;
  final bool _isOnline = true;
  final int _pendingSyncCount = 0;

  final List<Widget> _screens = const [
    DashboardScreen(),
    VentasComprasScreen(),
    FacturacionScreen(),
    DeclaracionesScreen(),
    RecordatoriosScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Image.asset(
          'assets/images/logo.png',
          height: 38,
          fit: BoxFit.contain,
        ),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 4),
            child: Center(
              child: Text(
                widget.session.username,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          ),
          IconButton(
            tooltip: 'Cerrar sesión',
            onPressed: widget.onLogout,
            icon: const Icon(Icons.logout),
          ),
        ],
      ),
      body: Column(
        children: [
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 16),
            color: _isOnline ? Colors.green.shade700 : Colors.red.shade700,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Icon(
                      _isOnline ? Icons.cloud_done : Icons.cloud_off,
                      size: 16,
                      color: Colors.white,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      _isOnline ? 'En línea (Sincronizado)' : 'Modo Sin Conexión',
                      style: const TextStyle(color: Colors.white, fontSize: 12),
                    ),
                  ],
                ),
                Text(
                  'Pendientes: $_pendingSyncCount',
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          ),
          Expanded(child: _screens[_currentIndex]),
        ],
      ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _currentIndex,
        onTap: (index) => setState(() => _currentIndex = index),
        type: BottomNavigationBarType.fixed,
        items: const [
          BottomNavigationBarItem(icon: Icon(Icons.dashboard), label: 'Dashboard'),
          BottomNavigationBarItem(icon: Icon(Icons.receipt), label: 'Operaciones'),
          BottomNavigationBarItem(icon: Icon(Icons.document_scanner), label: 'Facturación'),
          BottomNavigationBarItem(icon: Icon(Icons.assignment), label: 'Declaración'),
          BottomNavigationBarItem(icon: Icon(Icons.alarm), label: 'Calendario'),
        ],
      ),
    );
  }
}
