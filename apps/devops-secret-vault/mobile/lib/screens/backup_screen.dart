import 'package:flutter/material.dart';
import '../models/secret.dart';
import '../services/vault_service.dart';

class BackupScreen extends StatefulWidget {
  final VaultService vaultService;

  const BackupScreen({super.key, required this.vaultService});

  @override
  State<BackupScreen> createState() => _BackupScreenState();
}

class _BackupScreenState extends State<BackupScreen> {
  final TextEditingController _exportPasswordController =
      TextEditingController();
  final TextEditingController _exportPathController = TextEditingController(
      text: '/data/user/0/vault.internal/backups/mobile_vault.vaultbackup');

  final TextEditingController _importPasswordController =
      TextEditingController();
  final TextEditingController _importPathController = TextEditingController(
      text: '/data/user/0/vault.internal/backups/mobile_vault.vaultbackup');

  bool _overwriteExisting = false;
  bool _isProcessing = false;
  BackupSummary? _exportResult;
  BackupSummary? _importResult;
  String? _statusMessage;
  bool _isError = false;

  Future<void> _handleExport() async {
    final pwd = _exportPasswordController.text.trim();
    final path = _exportPathController.text.trim();
    if (pwd.isEmpty || path.isEmpty) {
      setState(() {
        _statusMessage = 'Password and destination path are required';
        _isError = true;
      });
      return;
    }

    setState(() {
      _isProcessing = true;
      _statusMessage = null;
      _isError = false;
    });

    try {
      final summary = await widget.vaultService.exportBackup(pwd, path);
      setState(() {
        _exportResult = summary;
        _isProcessing = false;
        _statusMessage =
            'Successfully exported ${summary.totalSecrets} secrets to encrypted container.';
      });
    } catch (e) {
      setState(() {
        _isProcessing = false;
        _statusMessage = 'Export failed: $e';
        _isError = true;
      });
    }
  }

  Future<void> _handleImport() async {
    final pwd = _importPasswordController.text.trim();
    final path = _importPathController.text.trim();
    if (pwd.isEmpty || path.isEmpty) {
      setState(() {
        _statusMessage = 'Password and backup file path are required';
        _isError = true;
      });
      return;
    }

    setState(() {
      _isProcessing = true;
      _statusMessage = null;
      _isError = false;
    });

    try {
      final summary = await widget.vaultService
          .importBackup(pwd, path, overwrite: _overwriteExisting);
      setState(() {
        _importResult = summary;
        _isProcessing = false;
        _statusMessage =
            'Successfully restored ${summary.totalSecrets} secrets from backup.';
      });
    } catch (e) {
      setState(() {
        _isProcessing = false;
        _statusMessage = 'Restore failed: $e';
        _isError = true;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0A0E1A),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Status Feedback Card
            if (_statusMessage != null) ...[
              Container(
                padding: const EdgeInsets.all(14),
                decoration: BoxDecoration(
                  color: _isError
                      ? Colors.red.withOpacity(0.12)
                      : const Color(0xFF00E676).withOpacity(0.12),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: _isError
                        ? Colors.red.withOpacity(0.3)
                        : const Color(0xFF00E676).withOpacity(0.3),
                  ),
                ),
                child: Row(
                  children: [
                    Icon(
                      _isError ? Icons.error_outline : Icons.check_circle_outline,
                      color: _isError
                          ? Colors.redAccent
                          : const Color(0xFF00E676),
                      size: 20,
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Text(
                        _statusMessage!,
                        style: TextStyle(
                          color: _isError
                              ? Colors.redAccent
                              : const Color(0xFF00E676),
                          fontSize: 13,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
            ],

            // Export Section
            Container(
              padding: const EdgeInsets.all(18),
              decoration: BoxDecoration(
                color: const Color(0xFF131D31),
                borderRadius: BorderRadius.circular(16),
                border: Border.all(color: Colors.white.withOpacity(0.08)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.cloud_upload_outlined,
                          color: Color(0xFF00E5FF), size: 22),
                      const SizedBox(width: 10),
                      const Text(
                        'Export Encrypted Backup',
                        style: TextStyle(
                          color: Colors.white,
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Creates an authenticated AES-256-GCM container hardened with Argon2id key derivation.',
                    style: TextStyle(
                      color: Colors.white.withOpacity(0.6),
                      fontSize: 12,
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _exportPathController,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    decoration: InputDecoration(
                      labelText: 'Output Container Path',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _exportPasswordController,
                    obscureText: true,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    decoration: InputDecoration(
                      labelText: 'Container Encryption Password',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  SizedBox(
                    width: double.infinity,
                    height: 44,
                    child: ElevatedButton(
                      onPressed: _isProcessing ? null : _handleExport,
                      style: ElevatedButton.styleFrom(
                        backgroundColor: const Color(0xFF00E5FF),
                        foregroundColor: const Color(0xFF0A0E1A),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(10),
                        ),
                      ),
                      child: _isProcessing
                          ? const SizedBox(
                              width: 20,
                              height: 20,
                              child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: Color(0xFF0A0E1A),
                              ),
                            )
                          : const Text(
                              'Generate Encrypted Backup',
                              style: TextStyle(fontWeight: FontWeight.bold),
                            ),
                    ),
                  ),
                  if (_exportResult != null) ...[
                    const SizedBox(height: 12),
                    Container(
                      padding: const EdgeInsets.all(10),
                      decoration: BoxDecoration(
                        color: const Color(0xFF0F172A),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        'SHA-256: ${_exportResult!.checksumSha256}',
                        style: TextStyle(
                          color: Colors.white.withOpacity(0.5),
                          fontSize: 10,
                          fontFamily: 'Courier',
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(height: 20),

            // Import Section
            Container(
              padding: const EdgeInsets.all(18),
              decoration: BoxDecoration(
                color: const Color(0xFF131D31),
                borderRadius: BorderRadius.circular(16),
                border: Border.all(color: Colors.white.withOpacity(0.08)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.cloud_download_outlined,
                          color: Color(0xFF00E676), size: 22),
                      const SizedBox(width: 10),
                      const Text(
                        'Restore from Backup',
                        style: TextStyle(
                          color: Colors.white,
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Verifies container integrity and restores items to local database.',
                    style: TextStyle(
                      color: Colors.white.withOpacity(0.6),
                      fontSize: 12,
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _importPathController,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    decoration: InputDecoration(
                      labelText: 'Backup Container File Path',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _importPasswordController,
                    obscureText: true,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    decoration: InputDecoration(
                      labelText: 'Container Decryption Password',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  SwitchListTile(
                    title: const Text(
                      'Overwrite Existing Items',
                      style: TextStyle(color: Colors.white, fontSize: 13),
                    ),
                    subtitle: Text(
                      'Replaces secrets with matching IDs or fingerprints',
                      style: TextStyle(
                        color: Colors.white.withOpacity(0.5),
                        fontSize: 11,
                      ),
                    ),
                    value: _overwriteExisting,
                    activeColor: const Color(0xFF00E676),
                    contentPadding: EdgeInsets.zero,
                    onChanged: (val) => setState(() => _overwriteExisting = val),
                  ),
                  const SizedBox(height: 12),
                  SizedBox(
                    width: double.infinity,
                    height: 44,
                    child: ElevatedButton(
                      onPressed: _isProcessing ? null : _handleImport,
                      style: ElevatedButton.styleFrom(
                        backgroundColor: const Color(0xFF00E676),
                        foregroundColor: const Color(0xFF0A0E1A),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(10),
                        ),
                      ),
                      child: const Text(
                        'Restore Secrets to Vault',
                        style: TextStyle(fontWeight: FontWeight.bold),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
