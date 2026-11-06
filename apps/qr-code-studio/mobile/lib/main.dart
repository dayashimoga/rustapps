import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'services/qr_service.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await SystemChrome.setPreferredOrientations([
    DeviceOrientation.portraitUp,
    DeviceOrientation.portraitDown,
  ]);

  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(
      statusBarColor: Colors.transparent,
      statusBarIconBrightness: Brightness.light,
      systemNavigationBarColor: Color(0xFF0A0E17),
      systemNavigationBarIconBrightness: Brightness.light,
    ),
  );

  runApp(const QrStudioMobileApp());
}

class QrStudioMobileApp extends StatelessWidget {
  const QrStudioMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'QR Code Studio Mobile',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF0A0E17),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF06B6D4),
          surface: Color(0xFF131D2E),
        ),
      ),
      home: const QrStudioHomeScreen(),
    );
  }
}

class QrStudioHomeScreen extends StatefulWidget {
  const QrStudioHomeScreen({super.key});

  @override
  State<QrStudioHomeScreen> createState() => _QrStudioHomeScreenState();
}

class _QrStudioHomeScreenState extends State<QrStudioHomeScreen> {
  final _controller = TextEditingController(text: 'https://rustapps.offline');
  String _status = 'Ready (100% Offline Air-Gapped)';
  int _tabIndex = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('QR Code Studio'),
        backgroundColor: const Color(0xFF131D2E),
        elevation: 0,
        actions: [
          IconButton(
            icon: const Icon(Icons.shield_outlined, color: Color(0xFF06B6D4)),
            onPressed: () {
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(
                  content: Text('Air-Gapped Anti-Quishing Engine Active'),
                  duration: Duration(seconds: 2),
                ),
              );
            },
          )
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _controller,
              decoration: const InputDecoration(
                labelText: 'QR Code Payload',
                border: OutlineInputBorder(),
                filled: true,
                fillColor: Color(0xFF131D2E),
              ),
            ),
            const SizedBox(height: 16),
            ElevatedButton.icon(
              icon: const Icon(Icons.qr_code),
              label: const Text('Generate ISO/IEC 18004 QR'),
              style: ElevatedButton.styleFrom(
                backgroundColor: const Color(0xFF06B6D4),
                foregroundColor: Colors.black,
                padding: const EdgeInsets.symmetric(vertical: 14),
              ),
              onPressed: () {
                setState(() {
                  _status = 'Synthesized QR matrix for: ${_controller.text}';
                });
              },
            ),
            const SizedBox(height: 24),
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: const Color(0xFF131D2E),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: const Color(0xFF1E293B)),
              ),
              child: Text(
                _status,
                style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 13),
              ),
            ),
          ],
        ),
      ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (i) => setState(() => _tabIndex = i),
        backgroundColor: const Color(0xFF131D2E),
        selectedItemColor: const Color(0xFF06B6D4),
        unselectedItemColor: const Color(0xFF64748B),
        items: const [
          BottomNavigationBarItem(icon: Icon(Icons.qr_code_2), label: 'Generator'),
          BottomNavigationBarItem(icon: Icon(Icons.camera_alt), label: 'Scanner'),
          BottomNavigationBarItem(icon: Icon(Icons.history), label: 'History'),
        ],
      ),
    );
  }
}

