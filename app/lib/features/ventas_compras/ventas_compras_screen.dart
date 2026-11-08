import 'package:flutter/material.dart';

class VentasComprasScreen extends StatelessWidget {
  const VentasComprasScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Ventas y Compras')),
      body: ListView(
        padding: const EdgeInsets.all(16.0),
        children: [
          Card(
            child: ListTile(
              title: const Text('Venta de Servicios'),
              subtitle: const Text('2026-11-05 - Base: \$100.00 | IVA: \$13.00'),
              trailing: const Text('\$113.00', style: TextStyle(fontWeight: FontWeight.bold)),
            ),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () {},
        child: const Icon(Icons.add),
      ),
    );
  }
}
