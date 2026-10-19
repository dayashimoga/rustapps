/// Hardware-backed Biometric Authentication Service
///
/// Wraps Android BiometricPrompt (Keystore StrongBox) & iOS LocalAuthentication (Secure Enclave).
/// Allows one-touch unlock with hardware security level checks.

import 'package:flutter/services.dart';
import 'package:local_auth/local_auth.dart';

enum BiometricSecurityStatus {
  available,
  notEnrolled,
  notAvailable,
  lockedOut,
  unknown
}

class BiometricService {
  final LocalAuthentication _auth;
  bool _mockMode = false;
  bool _mockAuthenticated = true;

  BiometricService({LocalAuthentication? auth, bool mockMode = false})
      : _auth = auth ?? LocalAuthentication(),
        _mockMode = mockMode;

  /// Determine if biometric hardware is present and operational
  Future<bool> isBiometricsSupported() async {
    if (_mockMode) return true;
    try {
      final canCheck = await _auth.canCheckBiometrics;
      final isSupported = await _auth.isDeviceSupported();
      return canCheck && isSupported;
    } on PlatformException {
      return false;
    }
  }

  /// Get list of available biometric modalities (fingerprint, face, iris)
  Future<List<BiometricType>> getAvailableBiometrics() async {
    if (_mockMode) {
      return [BiometricType.fingerprint, BiometricType.strong];
    }
    try {
      return await _auth.getAvailableBiometrics();
    } on PlatformException {
      return [];
    }
  }

  /// Authenticate user via Android BiometricPrompt or iOS FaceID / TouchID
  Future<bool> authenticate({
    String reason = 'Authenticate to unlock DevOps Secret Vault',
    bool biometricOnly = true,
  }) async {
    if (_mockMode) return _mockAuthenticated;

    try {
      final isSupported = await isBiometricsSupported();
      if (!isSupported) return false;

      return await _auth.authenticate(
        localizedReason: reason,
        options: AuthenticationOptions(
          biometricOnly: biometricOnly,
          stickyAuth: true,
          useErrorDialogs: true,
        ),
      );
    } on PlatformException catch (e) {
      // User cancelled or biometric authentication failed
      return false;
    }
  }

  /// Stop any active biometric authentication prompt
  Future<void> cancelAuthentication() async {
    if (_mockMode) return;
    try {
      await _auth.stopAuthentication();
    } on PlatformException {
      // Ignore
    }
  }
}
