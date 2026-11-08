import 'package:flutter/material.dart';

class DashboardScreen extends StatelessWidget {
  const DashboardScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(16.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Resumen Fiscal del Período',
            style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 16),
          Row(
            children: [
              Expanded(
                child: Card(
                  color: Colors.blue.shade50,
                  child: const Padding(
                    padding: EdgeInsets.all(16.0),
                    child: Column(
                      children: [
                        Text('IVA a Pagar', style: TextStyle(color: Colors.grey)),
                        SizedBox(height: 8),
                        Text('\$9.10', style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold)),
                      ],
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Card(
                  color: Colors.teal.shade50,
                  child: const Padding(
                    padding: EdgeInsets.all(16.0),
                    child: Column(
                      children: [
                        Text('Pago a Cuenta', style: TextStyle(color: Colors.grey)),
                        SizedBox(height: 8),
                        Text('\$2.63', style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold)),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),
          const Text(
            'Explicación en lenguaje simple:',
            style: TextStyle(fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 8),
          const Text(
            'Tus ventas del mes generaron \$19.50 de IVA débito y tus compras registraron \$10.40 de IVA crédito. La diferencia a cancelar a Hacienda es de \$9.10.',
          ),
        ],
      ),
    );
  }
}
