import 'package:flutter/material.dart';
import 'screens/log_viewer_screen.dart';
import 'services/ulv_service.dart';

void main() {
  runApp(const UniversalLogViewerApp());
}

class UniversalLogViewerApp extends StatelessWidget {
  const UniversalLogViewerApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final service = UlvService();

    return MaterialApp(
      title: 'Universal Log Viewer',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.dark,
        primaryColor: const Color(0xFF38BDF8),
        scaffoldBackgroundColor: const Color(0xFF0F141C),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF38BDF8),
          secondary: Color(0xFF818CF8),
          surface: Color(0xFF161B22),
          background: const Color(0xFF0F141C),
        ),
        useMaterial3: true,
      ),
      home: LogViewerScreen(service: service),
    );
  }
}

