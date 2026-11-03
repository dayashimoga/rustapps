/// High-level Vault Service providing asynchronous Dart API over native C-ABI FFI
///
/// Handles JSON marshaling, pointer allocations, memory freeing, and error mapping.
/// Supports Mock Mode for testing without native binaries.

import 'dart:convert';
import 'dart:ffi';
import 'package:ffi/ffi.dart';
import '../bindings/vault_ffi.dart';
import '../models/secret.dart';

class VaultService {
  VaultFfiBindings? _ffi;
  Pointer<VaultHandleOpaque>? _handle;
  bool _mockMode = false;

  // Mock state for tests or standalone mode
  final List<SecretMetadata> _mockSecrets = [];
  final Map<String, Map<String, dynamic>> _mockPayloads = {};
  bool _mockInitialized = false;
  bool _mockLocked = true;
  String? _mockPassword;

  VaultService({bool mockMode = false}) : _mockMode = mockMode;

  bool get isMockMode => _mockMode;
  bool get isOpen => _handle != null || _mockMode;

  /// Open or initialize vault database handle
  Future<void> open(String dbPath) async {
    if (_mockMode) {
      _mockInitialized = true;
      _mockLocked = true;
      return;
    }

    try {
      _ffi = VaultFfiBindings.load();
      final pathPtr = dbPath.toNativeUtf8();
      try {
        _handle = _ffi!.createHandle(pathPtr);
        if (_handle == nullptr) {
          throw Exception('Failed to allocate native vault handle');
        }
      } finally {
        calloc.free(pathPtr);
      }
    } catch (e) {
      throw StateError('Failed to initialize native DevOps Secret Vault FFI library: $e');
    }
  }

  /// Close and dispose native vault handle
  void close() {
    if (_handle != null && _ffi != null) {
      _ffi!.freeHandle(_handle!);
      _handle = null;
    }
  }

  /// Check whether master key and salt exist in database
  Future<bool> isInitialized() async {
    if (_mockMode) return _mockInitialized;
    if (_handle == null) throw StateError('Vault handle not opened');
    return _ffi!.isInitialized(_handle!) != 0;
  }

  /// Initialize fresh vault with master password
  Future<void> initialize(String masterPassword) async {
    if (_mockMode) {
      _mockPassword = masterPassword;
      _mockInitialized = true;
      _mockLocked = false;
      return;
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final pwdPtr = masterPassword.toNativeUtf8();
    try {
      final res = _ffi!.initialize(_handle!, pwdPtr);
      if (res != 0) {
        throw Exception('Vault initialization failed with status code $res');
      }
    } finally {
      calloc.free(pwdPtr);
    }
  }

  /// Unlock vault using master password
  Future<bool> unlock(String masterPassword) async {
    if (_mockMode) {
      if (_mockPassword != null && _mockPassword == masterPassword) {
        _mockLocked = false;
        return true;
      }
      return false;
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final pwdPtr = masterPassword.toNativeUtf8();
    try {
      final res = _ffi!.unlock(_handle!, pwdPtr);
      return res == 0;
    } finally {
      calloc.free(pwdPtr);
    }
  }

  /// Zeroize master key in memory and lock vault
  Future<void> lock() async {
    if (_mockMode) {
      _mockLocked = true;
      return;
    }
    if (_handle == null) return;
    _ffi!.lock(_handle!);
  }

  /// Check whether vault is currently locked
  Future<bool> isLocked() async {
    if (_mockMode) return _mockLocked;
    if (_handle == null) return true;
    return _ffi!.isLocked(_handle!) != 0;
  }

  /// Search secrets using full-text or category query
  Future<List<SecretMetadata>> searchSecrets(String query,
      {int limit = 50}) async {
    if (_mockMode) {
      if (query.isEmpty || query == '*') return List.of(_mockSecrets);
      final q = query.toLowerCase();
      return _mockSecrets
          .where((s) =>
              s.title.toLowerCase().contains(q) ||
              s.tags.any((t) => t.toLowerCase().contains(q)) ||
              s.secretType.toLowerCase().contains(q))
          .take(limit)
          .toList();
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final queryPtr = query.toNativeUtf8();
    try {
      final resPtr = _ffi!.search(_handle!, queryPtr, limit);
      final jsonStr = _ffi!.readAndFreeString(resPtr);
      if (jsonStr == null) return [];

      final list = jsonDecode(jsonStr) as List<dynamic>;
      return list
          .map((item) => SecretMetadata.fromJson(item as Map<String, dynamic>))
          .toList();
    } finally {
      calloc.free(queryPtr);
    }
  }

  /// Create a new secret
  Future<SecretMetadata> createSecret({
    required String title,
    required String secretType,
    required Map<String, dynamic> data,
    String? description,
    List<String> tags = const [],
    DateTime? expiresAt,
  }) async {
    if (_mockMode) {
      final id = 'mock-${DateTime.now().millisecondsSinceEpoch}';
      final meta = SecretMetadata(
        id: id,
        title: title,
        description: description,
        secretType: secretType,
        tags: tags,
        createdAt: DateTime.now(),
        updatedAt: DateTime.now(),
        expiresAt: expiresAt,
        version: 1,
        entropyScore: 85.0,
        accessCount: 0,
      );
      _mockSecrets.insert(0, meta);
      _mockPayloads[id] = {'type': secretType, 'data': data};
      return meta;
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final payloadMap = {
      'title': title,
      if (description != null) 'description': description,
      'tags': tags,
      if (expiresAt != null) 'expires_at': expiresAt.toIso8601String(),
      'payload': {
        'type': secretType,
        'data': data,
      },
    };

    final jsonStr = jsonEncode(payloadMap);
    final jsonPtr = jsonStr.toNativeUtf8();
    try {
      final resPtr = _ffi!.createSecret(_handle!, jsonPtr);
      final outStr = _ffi!.readAndFreeString(resPtr);
      if (outStr == null) {
        throw Exception('Native secret creation returned null');
      }
      return SecretMetadata.fromJson(
          jsonDecode(outStr) as Map<String, dynamic>);
    } finally {
      calloc.free(jsonPtr);
    }
  }

  /// Retrieve and decrypt a secret
  Future<DecryptedSecret> getSecret(String id) async {
    if (_mockMode) {
      final meta = _mockSecrets.firstWhere((s) => s.id == id,
          orElse: () => throw Exception('Secret not found: $id'));
      final p = _mockPayloads[id] ?? {'type': meta.secretType, 'data': {}};
      return DecryptedSecret(metadata: meta, payload: p);
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final idPtr = id.toNativeUtf8();
    try {
      final resPtr = _ffi!.getSecret(_handle!, idPtr);
      final outStr = _ffi!.readAndFreeString(resPtr);
      if (outStr == null) {
        throw Exception('Failed to decrypt secret $id');
      }
      return DecryptedSecret.fromJson(
          jsonDecode(outStr) as Map<String, dynamic>);
    } finally {
      calloc.free(idPtr);
    }
  }

  /// Generate RFC 6238 TOTP code
  Future<TotpCodeResponse> generateTotp(String secretId) async {
    if (_mockMode) {
      final sec = 30 - (DateTime.now().second % 30);
      return TotpCodeResponse(
        secretId: secretId,
        code: '849201',
        secondsRemaining: sec,
        periodSeconds: 30,
      );
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final idPtr = secretId.toNativeUtf8();
    try {
      final resPtr = _ffi!.generateTotp(_handle!, idPtr);
      final outStr = _ffi!.readAndFreeString(resPtr);
      if (outStr == null) {
        throw Exception('Failed to generate TOTP code for $secretId');
      }
      return TotpCodeResponse.fromJson(
          jsonDecode(outStr) as Map<String, dynamic>);
    } finally {
      calloc.free(idPtr);
    }
  }

  /// Generate comprehensive 5-dimensional security risk report
  Future<VaultHealthReport> getHealthReport() async {
    if (_mockMode) {
      return VaultHealthReport(
        overallScore: 92.5,
        grade: 'A',
        totalSecrets: _mockSecrets.length,
        dimensionBreakdown: {
          'entropy': 95.0,
          'uniqueness': 90.0,
          'rotation': 88.0,
          'expiration': 100.0,
          'mfa_coverage': 89.0,
        },
        findings: [
          SecurityAuditFinding(
            findingType: 'KeyRotation',
            severity: 'low',
            secretId: _mockSecrets.isNotEmpty ? _mockSecrets.first.id : 'none',
            secretTitle:
                _mockSecrets.isNotEmpty ? _mockSecrets.first.title : 'Root Key',
            message: 'Secret has not been rotated in the last 90 days.',
            remediationAdvice: 'Rotate credentials periodically.',
          ),
        ],
      );
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final resPtr = _ffi!.getHealthReport(_handle!);
    final outStr = _ffi!.readAndFreeString(resPtr);
    if (outStr == null) {
      throw Exception('Failed to generate health report');
    }
    return VaultHealthReport.fromJson(
        jsonDecode(outStr) as Map<String, dynamic>);
  }

  /// Inspect and extract TLS X.509 certificate metadata
  Future<Map<String, dynamic>> inspectCertificate(String certPem) async {
    if (_mockMode) {
      return {
        'subject': 'CN=api.vault.internal',
        'issuer': "Let's Encrypt",
        'valid_days_remaining': 72,
        'is_expired': false,
        'san': ['api.vault.internal', 'vault.internal'],
      };
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final pemPtr = certPem.toNativeUtf8();
    try {
      final resPtr = _ffi!.inspectCertificate(_handle!, pemPtr);
      final outStr = _ffi!.readAndFreeString(resPtr);
      if (outStr == null) {
        throw Exception('Failed to inspect certificate');
      }
      return jsonDecode(outStr) as Map<String, dynamic>;
    } finally {
      calloc.free(pemPtr);
    }
  }

  /// Export encrypted backup
  Future<BackupSummary> exportBackup(
      String backupPassword, String outputPath) async {
    if (_mockMode) {
      return BackupSummary(
        totalSecrets: _mockSecrets.length,
        backupTimestamp: DateTime.now(),
        checksumSha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
      );
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final pwdPtr = backupPassword.toNativeUtf8();
    final pathPtr = outputPath.toNativeUtf8();
    try {
      final resPtr = _ffi!.exportBackup(_handle!, pwdPtr, pathPtr);
      final outStr = _ffi!.readAndFreeString(resPtr);
      if (outStr == null) {
        throw Exception('Backup export failed');
      }
      return BackupSummary.fromJson(jsonDecode(outStr) as Map<String, dynamic>);
    } finally {
      calloc.free(pwdPtr);
      calloc.free(pathPtr);
    }
  }

  /// Import encrypted backup
  Future<BackupSummary> importBackup(
      String backupPassword, String inputPath,
      {bool overwrite = false}) async {
    if (_mockMode) {
      return BackupSummary(
        totalSecrets: 10,
        backupTimestamp: DateTime.now(),
        checksumSha256: 'mock-restore-checksum',
      );
    }
    if (_handle == null) throw StateError('Vault handle not opened');

    final pwdPtr = backupPassword.toNativeUtf8();
    final pathPtr = inputPath.toNativeUtf8();
    try {
      final resPtr =
          _ffi!.importBackup(_handle!, pwdPtr, pathPtr, overwrite ? 1 : 0);
      final outStr = _ffi!.readAndFreeString(resPtr);
      if (outStr == null) {
        throw Exception('Backup import failed');
      }
      return BackupSummary.fromJson(jsonDecode(outStr) as Map<String, dynamic>);
    } finally {
      calloc.free(pwdPtr);
      calloc.free(pathPtr);
    }
  }

  /// Verify tamper-evident cryptographic hash chain of audit logs
  Future<bool> verifyAuditChain() async {
    if (_mockMode) return true;
    if (_handle == null) throw StateError('Vault handle not opened');
    return _ffi!.verifyAuditChain(_handle!) == 1;
  }

  /// Check whether biometric authentication is supported on this mobile device
  Future<bool> isBiometricAvailable() async {
    if (_mockMode) return true;
    if (_handle == null) return false;
    return _ffi!.isBiometricAvailable(_handle!) == 1;
  }

  /// Enable biometric unlock by saving active master key in platform keyring
  Future<bool> enableBiometricUnlock() async {
    if (_mockMode) return true;
    if (_handle == null) throw StateError('Vault handle not opened');
    return _ffi!.enableBiometricUnlock(_handle!) == 0;
  }

  /// Disable biometric unlock by clearing key from platform keyring
  Future<bool> disableBiometricUnlock() async {
    if (_mockMode) return true;
    if (_handle == null) throw StateError('Vault handle not opened');
    return _ffi!.disableBiometricUnlock(_handle!) == 0;
  }

  /// Unlock vault with native platform biometrics
  Future<bool> unlockWithBiometrics() async {
    if (_mockMode) {
      _mockLocked = false;
      return true;
    }
    if (_handle == null) throw StateError('Vault handle not opened');
    final res = _ffi!.unlockWithBiometrics(_handle!);
    return res == 0;
  }

  /// Split vault master key into Shamir shares JSON
  Future<String> splitMasterKey(int threshold, int totalShares) async {
    if (_mockMode) {
      return jsonEncode([
        {'index': 1, 'threshold': threshold, 'total_shares': totalShares, 'data': [1, 2, 3]},
        {'index': 2, 'threshold': threshold, 'total_shares': totalShares, 'data': [4, 5, 6]},
      ]);
    }
    if (_handle == null) throw StateError('Vault handle not opened');
    final resPtr = _ffi!.splitMasterKey(_handle!, threshold, totalShares);
    final outStr = _ffi!.readAndFreeString(resPtr);
    if (outStr == null) {
      throw Exception('Failed to split master key');
    }
    return outStr;
  }

  /// Unlock vault using Shamir shares JSON
  Future<bool> unlockWithShamirShares(String sharesJson) async {
    if (_mockMode) {
      _mockLocked = false;
      return true;
    }
    if (_handle == null) throw StateError('Vault handle not opened');
    final jsonPtr = sharesJson.toNativeUtf8();
    try {
      final res = _ffi!.unlockWithShamirShares(_handle!, jsonPtr);
      return res == 0;
    } finally {
      calloc.free(jsonPtr);
    }
  }
}
