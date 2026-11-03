import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'services/afe_service.dart';
import 'screens/browser_screen.dart';
import 'screens/content_search_screen.dart';
import 'screens/ocr_screen.dart';
import 'screens/compliance_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final afeService = AfeService();
  await afeService.open();

  runApp(
    MultiProvider(
      providers: [
        Provider<AfeService>.value(value: afeService),
      ],
      child: const AiFileExplorerApp(),
    ),
  );
}

class AiFileExplorerApp extends StatelessWidget {
  const AiFileExplorerApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'AI File Explorer',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF00E5FF),
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
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

  @override
  Widget build(BuildContext context) {
    final service = Provider.of<AfeService>(context);

    final screens = [
      BrowserScreen(service: service),
      ContentSearchScreen(service: service),
      OcrScreen(service: service),
      ComplianceScreen(service: service),
    ];

    return Scaffold(
      body: IndexedStack(
        index: _currentIndex,
        children: screens,
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _currentIndex,
        onDestinationSelected: (index) {
          setState(() => _currentIndex = index);
        },
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.folder_outlined),
            selectedIcon: Icon(Icons.folder),
            label: 'Explorer',
          ),
          NavigationDestination(
            icon: Icon(Icons.search_outlined),
            selectedIcon: Icon(Icons.search),
            label: 'Search',
          ),
          NavigationDestination(
            icon: Icon(Icons.document_scanner_outlined),
            selectedIcon: Icon(Icons.document_scanner),
            label: 'OCR',
          ),
          NavigationDestination(
            icon: Icon(Icons.shield_outlined),
            selectedIcon: Icon(Icons.shield),
            label: 'Compliance',
          ),
        ],
      ),
    );
  }
}

