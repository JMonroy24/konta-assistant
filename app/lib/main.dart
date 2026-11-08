import 'package:flutter/material.dart';
import 'core/theme.dart';
import 'features/dashboard/dashboard_screen.dart';
import 'features/ventas_compras/ventas_compras_screen.dart';
import 'features/facturacion/facturacion_screen.dart';
import 'features/declaraciones/declaraciones_screen.dart';
import 'features/recordatorios/recordatorios_screen.dart';

void main() {
  runApp(const KontaApp());
}

class KontaApp extends StatelessWidget {
  const KontaApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Konta Assistant',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.lightTheme,
      home: const MainLayoutScreen(),
    );
  }
}

class MainLayoutScreen extends StatefulWidget {
  const MainLayoutScreen({super.key});

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
      ),
      body: Column(
        children: [
          // Franja visible de estado de sincronización y conectividad
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
