import 'package:flutter/material.dart';
import '../services/vault_service.dart';
import '../services/biometric_service.dart';
import 'vault_home_screen.dart';

class UnlockScreen extends StatefulWidget {
  final VaultService vaultService;
  final BiometricService biometricService;

  const UnlockScreen({
    super.key,
    required this.vaultService,
    required this.biometricService,
  });

  @override
  State<UnlockScreen> createState() => _UnlockScreenState();
}

class _UnlockScreenState extends State<UnlockScreen> {
  final TextEditingController _passwordController = TextEditingController();
  final TextEditingController _confirmController = TextEditingController();

  bool _isInitialized = false;
  bool _isLoading = true;
  bool _obscurePassword = true;
  bool _canUseBiometrics = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _checkVaultStatus();
  }

  Future<void> _checkVaultStatus() async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      if (!widget.vaultService.isOpen) {
        await widget.vaultService.open(':memory:');
      }

      final initialized = await widget.vaultService.isInitialized();
      final hasBiometrics =
          await widget.biometricService.isBiometricsSupported();

      setState(() {
        _isInitialized = initialized;
        _canUseBiometrics = hasBiometrics;
        _isLoading = false;
      });

      if (initialized && hasBiometrics) {
        _triggerBiometricUnlock();
      }
    } catch (e) {
      setState(() {
        _isLoading = false;
        _errorMessage = 'Status check failed: $e';
      });
    }
  }

  Future<void> _triggerBiometricUnlock() async {
    final success = await widget.biometricService.authenticate();
    if (success && mounted) {
      // In production, biometrics decrypts Master Key from Keystore.
      // In this flow, we unlock using biometric authorization.
      final unlocked = await widget.vaultService.unlock('biometric-auth-key');
      if (unlocked && mounted) {
        _navigateToHome();
      }
    }
  }

  Future<void> _handlePasswordSubmit() async {
    final pwd = _passwordController.text.trim();
    if (pwd.isEmpty) {
      setState(() => _errorMessage = 'Password cannot be empty');
      return;
    }

    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      if (!_isInitialized) {
        final confirm = _confirmController.text.trim();
        if (pwd != confirm) {
          setState(() {
            _isLoading = false;
            _errorMessage = 'Passwords do not match';
          });
          return;
        }
        await widget.vaultService.initialize(pwd);
        if (mounted) _navigateToHome();
      } else {
        final success = await widget.vaultService.unlock(pwd);
        if (success && mounted) {
          _navigateToHome();
        } else {
          setState(() {
            _isLoading = false;
            _errorMessage = 'Invalid master password';
          });
        }
      }
    } catch (e) {
      setState(() {
        _isLoading = false;
        _errorMessage = 'Unlock error: $e';
      });
    }
  }

  void _navigateToHome() {
    Navigator.of(context).pushReplacement(
      MaterialPageRoute(
        builder: (_) => VaultHomeScreen(
          vaultService: widget.vaultService,
          biometricService: widget.biometricService,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0A0E1A),
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.symmetric(horizontal: 28.0),
            child: _isLoading
                ? const CircularProgressIndicator(color: Color(0xFF00E5FF))
                : Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      // Vault Icon Shield
                      Container(
                        padding: const EdgeInsets.all(20),
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          color: const Color(0xFF131D31),
                          border: Border.all(
                            color: const Color(0xFF00E5FF).withOpacity(0.3),
                            width: 1.5,
                          ),
                          boxShadow: [
                            BoxShadow(
                              color: const Color(0xFF00E5FF).withOpacity(0.15),
                              blurRadius: 24,
                              spreadRadius: 2,
                            ),
                          ],
                        ),
                        child: const Icon(
                          Icons.shield_outlined,
                          size: 56,
                          color: Color(0xFF00E5FF),
                        ),
                      ),
                      const SizedBox(height: 24),

                      // Title
                      const Text(
                        'DevOps Secret Vault',
                        style: TextStyle(
                          fontSize: 26,
                          fontWeight: FontWeight.bold,
                          color: Colors.white,
                          letterSpacing: 0.5,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Text(
                        _isInitialized
                            ? 'Zero-Knowledge Offline Security'
                            : 'Initialize New Master Vault Key',
                        style: TextStyle(
                          fontSize: 14,
                          color: Colors.white.withOpacity(0.6),
                        ),
                      ),
                      const SizedBox(height: 36),

                      if (_errorMessage != null) ...[
                        Container(
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: Colors.red.withOpacity(0.1),
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(
                              color: Colors.red.withOpacity(0.3),
                            ),
                          ),
                          child: Text(
                            _errorMessage!,
                            style: const TextStyle(
                              color: Color(0xFFFF5252),
                              fontSize: 13,
                            ),
                            textAlign: TextAlign.center,
                          ),
                        ),
                        const SizedBox(height: 16),
                      ],

                      // Password Field
                      Container(
                        decoration: BoxDecoration(
                          color: const Color(0xFF131D31),
                          borderRadius: BorderRadius.circular(12),
                          border: Border.all(
                            color: Colors.white.withOpacity(0.1),
                          ),
                        ),
                        child: TextField(
                          controller: _passwordController,
                          obscureText: _obscurePassword,
                          style: const TextStyle(color: Colors.white),
                          decoration: InputDecoration(
                            hintText: _isInitialized
                                ? 'Master Password'
                                : 'Set Master Password (min 12 chars)',
                            hintStyle: TextStyle(
                              color: Colors.white.withOpacity(0.4),
                              fontSize: 14,
                            ),
                            prefixIcon: const Icon(
                              Icons.lock_outline,
                              color: Color(0xFF00E5FF),
                              size: 20,
                            ),
                            suffixIcon: IconButton(
                              icon: Icon(
                                _obscurePassword
                                    ? Icons.visibility_off
                                    : Icons.visibility,
                                color: Colors.white.withOpacity(0.5),
                                size: 20,
                              ),
                              onPressed: () {
                                setState(() {
                                  _obscurePassword = !_obscurePassword;
                                });
                              },
                            ),
                            border: InputBorder.none,
                            contentPadding: const EdgeInsets.symmetric(
                              horizontal: 16,
                              vertical: 16,
                            ),
                          ),
                        ),
                      ),

                      if (!_isInitialized) ...[
                        const SizedBox(height: 16),
                        Container(
                          decoration: BoxDecoration(
                            color: const Color(0xFF131D31),
                            borderRadius: BorderRadius.circular(12),
                            border: Border.all(
                              color: Colors.white.withOpacity(0.1),
                            ),
                          ),
                          child: TextField(
                            controller: _confirmController,
                            obscureText: _obscurePassword,
                            style: const TextStyle(color: Colors.white),
                            decoration: InputDecoration(
                              hintText: 'Confirm Master Password',
                              hintStyle: TextStyle(
                                color: Colors.white.withOpacity(0.4),
                                fontSize: 14,
                              ),
                              prefixIcon: const Icon(
                                Icons.lock_reset,
                                color: Color(0xFF00E5FF),
                                size: 20,
                              ),
                              border: InputBorder.none,
                              contentPadding: const EdgeInsets.symmetric(
                                horizontal: 16,
                                vertical: 16,
                              ),
                            ),
                          ),
                        ),
                      ],

                      const SizedBox(height: 24),

                      // Submit Button
                      SizedBox(
                        width: double.infinity,
                        height: 50,
                        child: ElevatedButton(
                          onPressed: _handlePasswordSubmit,
                          style: ElevatedButton.styleFrom(
                            backgroundColor: const Color(0xFF00E5FF),
                            foregroundColor: const Color(0xFF0A0E1A),
                            shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(12),
                            ),
                            elevation: 0,
                          ),
                          child: Text(
                            _isInitialized ? 'Unlock Vault' : 'Create Vault',
                            style: const TextStyle(
                              fontSize: 16,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                        ),
                      ),

                      if (_isInitialized && _canUseBiometrics) ...[
                        const SizedBox(height: 20),
                        OutlinedButton.icon(
                          onPressed: _triggerBiometricUnlock,
                          icon: const Icon(
                            Icons.fingerprint,
                            color: Color(0xFF00E5FF),
                            size: 24,
                          ),
                          label: const Text(
                            'Quick Biometric Unlock',
                            style: TextStyle(
                              color: Colors.white,
                              fontSize: 14,
                            ),
                          ),
                          style: OutlinedButton.styleFrom(
                            side: Border.all(
                              color: const Color(0xFF00E5FF).withOpacity(0.3),
                            ),
                            shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(12),
                            ),
                            padding: const EdgeInsets.symmetric(
                              horizontal: 20,
                              vertical: 12,
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
          ),
        ),
      ),
    );
  }
}
