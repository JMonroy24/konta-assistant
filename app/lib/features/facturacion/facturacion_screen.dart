import 'package:flutter/material.dart';

class FacturacionScreen extends StatelessWidget {
  const FacturacionScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Facturación Electrónica (DTE)')),
      body: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(12),
              color: Colors.amber.shade100,
              child: const Row(
                children: [
                  Icon(Icons.info_outline, color: Colors.orange),
                  SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'MODO SIMULADOR: Los DTEs emitidos son validados contra el simulador de pruebas y no tienen validez legal oficial.',
                      style: TextStyle(fontSize: 12),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            const Text('Facturas Emitidas', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
            const SizedBox(height: 8),
            Card(
              child: ListTile(
                leading: const Icon(Icons.receipt_long, color: Colors.green),
                title: const Text('DTE-01-M001-00000001'),
                subtitle: const Text('Estado: Aceptado Simulado (Hacienda Mock)'),
                trailing: const Chip(
                  label: Text('SIMULADO', style: TextStyle(fontSize: 10, color: Colors.white)),
                  backgroundColor: Colors.teal,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
