import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'screens/text_translate_screen.dart';
import 'screens/conversation_screen.dart';
import 'screens/camera_translate_screen.dart';
import 'screens/phrasebook_screen.dart';
import 'services/ult_service.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final service = UltService();
  await service.initialize();

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider<UltService>.value(value: service),
      ],
      child: const UltApp(),
    ),
  );
}

class UltApp extends StatelessWidget {
  const UltApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Universal Live Translator',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF060913),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF0B1220),
          elevation: 0,
        ),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF6366F1),
          secondary: Color(0xFF06B6D4),
        ),
      ),
      home: const MainNavigationScreen(),
    );
  }
}

class MainNavigationScreen extends StatefulWidget {
  const MainNavigationScreen({super.key});

  @override
  State<MainNavigationScreen> createState() => _MainNavigationScreenState();
}

class _MainNavigationScreenState extends State<MainNavigationScreen> {
  int _currentIndex = 0;

  final List<Widget> _screens = const [
    TextTranslateScreen(),
    ConversationScreen(),
    CameraTranslateScreen(),
    PhrasebookScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: _screens[_currentIndex],
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _currentIndex,
        onTap: (index) => setState(() => _currentIndex = index),
        type: BottomNavigationBarType.fixed,
        backgroundColor: const Color(0xFF0B1220),
        selectedItemColor: const Color(0xFF6366F1),
        unselectedItemColor: Colors.white54,
        items: const [
          BottomNavigationBarItem(
            icon: Icon(Icons.translate),
            label: 'Text',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.record_voice_over),
            label: 'Voice',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.camera_alt),
            label: 'Camera',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.book),
            label: 'Phrasebook',
          ),
        ],
      ),
    );
  }
}

