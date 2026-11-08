import 'package:flutter/material.dart';

class DeclaracionesScreen extends StatelessWidget {
  const DeclaracionesScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Borrador de Declaraciones')),
      body: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          children: [
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: const [
                    Text('Declaración Mensual de IVA (F-07)', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
                    SizedBox(height: 8),
                    Text('Período: Noviembre 2026'),
                    Text('Débito Fiscal: \$19.50'),
                    Text('Crédito Fiscal: \$10.40'),
                    Divider(),
                    Text('Total a Pagar: \$9.10', style: TextStyle(fontWeight: FontWeight.bold, color: Colors.blue)),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
