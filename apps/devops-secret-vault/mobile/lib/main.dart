import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'services/vault_service.dart';
import 'services/biometric_service.dart';
import 'screens/unlock_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Enforce portrait orientation and edge-to-edge transparent system bars
  await SystemChrome.setPreferredOrientations([
    DeviceOrientation.portraitUp,
    DeviceOrientation.portraitDown,
  ]);

  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(
      statusBarColor: Colors.transparent,
      statusBarIconBrightness: Brightness.light,
      systemNavigationBarColor: Color(0xFF0F172A),
      systemNavigationBarIconBrightness: Brightness.light,
    ),
  );

  final vaultService = VaultService();
  final biometricService = BiometricService();

  runApp(DevOpsSecretVaultApp(
    vaultService: vaultService,
    biometricService: biometricService,
  ));
}

class DevOpsSecretVaultApp extends StatelessWidget {
  final VaultService vaultService;
  final BiometricService biometricService;

  const DevOpsSecretVaultApp({
    super.key,
    required this.vaultService,
    required this.biometricService,
  });

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'DevOps Secret Vault Mobile',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData(
        brightness: Brightness.dark,
        scaffoldBackgroundColor: const Color(0xFF0A0E1A),
        primaryColor: const Color(0xFF00E5FF),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF00E5FF),
          secondary: Color(0xFF8B5CF6),
          surface: Color(0xFF131D31),
          background: Color(0xFF0A0E1A),
          error: Color(0xFFFF5252),
          onPrimary: Color(0xFF0A0E1A),
          onSurface: Colors.white,
          onBackground: Colors.white,
        ),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF0F172A),
          elevation: 0,
          centerTitle: false,
          iconTheme: IconThemeData(color: Colors.white),
        ),
        useMaterial3: true,
      ),
      home: UnlockScreen(
        vaultService: vaultService,
        biometricService: biometricService,
      ),
    );
  }
}
