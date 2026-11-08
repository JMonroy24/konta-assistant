import 'package:flutter/material.dart';

class RecordatoriosScreen extends StatelessWidget {
  const RecordatoriosScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Calendario y Recordatorios')),
      body: ListView(
        padding: const EdgeInsets.all(16.0),
        children: const [
          Card(
            child: ListTile(
              leading: Icon(Icons.alarm, color: Colors.red),
              title: Text('Vencimiento IVA F-07'),
              subtitle: Text('Fecha límite: 10 de Diciembre de 2026'),
              trailing: Chip(label: Text('Pendiente')),
            ),
          ),
          Card(
            child: ListTile(
              leading: Icon(Icons.alarm, color: Colors.amber),
              title: Text('Pago a Cuenta Mensual'),
              subtitle: Text('Fecha límite: 10 de Diciembre de 2026'),
              trailing: Chip(label: Text('Pendiente')),
            ),
          ),
        ],
      ),
    );
  }
}
