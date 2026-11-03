/// Professional Audio & Video Editor - Mobile Entry Point
///
/// 100% Offline Multi-Track NLE, Hardware Color Grading & Audio Suite.

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'services/ave_service.dart';
import 'screens/ave_studio_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final aveService = AveService();
  try {
    await aveService.initialize();
  } catch (e) {
    debugPrint('Native AveService initialization note: $e');
  }

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider<AveService>.value(value: aveService),
      ],
      child: const ProAvEditorApp(),
    ),
  );
}

class ProAvEditorApp extends StatelessWidget {
  const ProAvEditorApp({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Pro AV Editor',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark().copyWith(
        primaryColor: const Color(0xFF0284C7),
        scaffoldBackgroundColor: const Color(0xFF0D1117),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF0284C7),
          secondary: Color(0xFF38BDF8),
        ),
      ),
      home: const AveStudioScreen(),
    );
  }
}

