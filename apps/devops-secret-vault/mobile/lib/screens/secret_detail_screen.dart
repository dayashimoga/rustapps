import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/secret.dart';
import '../services/vault_service.dart';
import 'totp_screen.dart';

class SecretDetailScreen extends StatefulWidget {
  final VaultService vaultService;
  final String secretId;

  const SecretDetailScreen({
    super.key,
    required this.vaultService,
    required this.secretId,
  });

  @override
  State<SecretDetailScreen> createState() => _SecretDetailScreenState();
}

class _SecretDetailScreenState extends State<SecretDetailScreen> {
  DecryptedSecret? _secret;
  bool _isLoading = true;
  String? _error;
  final Set<String> _revealedFields = {};
  Timer? _clipboardSanitizeTimer;

  @override
  void initState() {
    super.initState();
    _loadSecret();
  }

  @override
  void dispose() {
    _clipboardSanitizeTimer?.cancel();
    super.dispose();
  }

  Future<void> _loadSecret() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });

    try {
      final s = await widget.vaultService.getSecret(widget.secretId);
      setState(() {
        _secret = s;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _error = 'Failed to load secret: $e';
        _isLoading = false;
      });
    }
  }

  void _copyToClipboard(String label, String value) {
    Clipboard.setData(ClipboardData(text: value));

    _clipboardSanitizeTimer?.cancel();
    _clipboardSanitizeTimer = Timer(const Duration(seconds: 45), () {
      Clipboard.setData(const ClipboardData(text: ''));
    });

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('$label copied to clipboard. Auto-clearing in 45s.'),
        backgroundColor: const Color(0xFF1E293B),
        behavior: SnackBarBehavior.floating,
        duration: const Duration(seconds: 3),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0A0E1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F172A),
        elevation: 0,
        title: Text(
          _secret?.metadata.title ?? 'Secret Details',
          style: const TextStyle(color: Colors.white, fontSize: 17),
        ),
        actions: [
          if (_secret?.metadata.secretType == 'totp')
            IconButton(
              icon: const Icon(Icons.pin, color: Color(0xFF00E5FF)),
              tooltip: 'Generate TOTP',
              onPressed: () {
                Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) =>
                        TotpScreen(vaultService: widget.vaultService),
                  ),
                );
              },
            ),
        ],
      ),
      body: _isLoading
          ? const Center(
              child: CircularProgressIndicator(color: Color(0xFF00E5FF)))
          : _error != null
              ? Center(
                  child: Text(
                    _error!,
                    style: const TextStyle(color: Colors.redAccent),
                  ),
                )
              : _buildContent(),
    );
  }

  Widget _buildContent() {
    final meta = _secret!.metadata;
    final payload = _secret!.payloadData;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(18.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header Card
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: const Color(0xFF131D31),
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: Colors.white.withOpacity(0.08)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      meta.title,
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 20,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: const Color(0xFF00E5FF).withOpacity(0.12),
                        borderRadius: BorderRadius.circular(6),
                      ),
                      child: Text(
                        meta.secretType.replaceAll('_', ' ').toUpperCase(),
                        style: const TextStyle(
                          color: Color(0xFF00E5FF),
                          fontSize: 10,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ],
                ),
                if (meta.description != null && meta.description!.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Text(
                    meta.description!,
                    style: TextStyle(
                      color: Colors.white.withOpacity(0.7),
                      fontSize: 13,
                    ),
                  ),
                ],
                const SizedBox(height: 12),
                Row(
                  children: [
                    Icon(Icons.history,
                        size: 14, color: Colors.white.withOpacity(0.4)),
                    const SizedBox(width: 4),
                    Text(
                      'Version ${meta.version} • Accessed ${meta.accessCount}x',
                      style: TextStyle(
                        color: Colors.white.withOpacity(0.4),
                        fontSize: 12,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 18),

          // Decrypted Payload Fields
          const Text(
            'SECURE CREDENTIAL PAYLOAD',
            style: TextStyle(
              color: Color(0xFF00E5FF),
              fontSize: 11,
              fontWeight: FontWeight.bold,
              letterSpacing: 1.0,
            ),
          ),
          const SizedBox(height: 10),

          ...payload.entries.map((entry) {
            final key = entry.key;
            final val = entry.value.toString();
            final isRevealed = _revealedFields.contains(key);

            return Container(
              margin: const EdgeInsets.only(bottom: 12),
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
              decoration: BoxDecoration(
                color: const Color(0xFF131D31),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: Colors.white.withOpacity(0.06)),
              ),
              child: Row(
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          key.replaceAll('_', ' ').toUpperCase(),
                          style: TextStyle(
                            color: Colors.white.withOpacity(0.5),
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          isRevealed ? val : '••••••••••••••••',
                          style: TextStyle(
                            color: Colors.white,
                            fontSize: 14,
                            fontFamily: isRevealed ? null : 'Courier',
                            letterSpacing: isRevealed ? 0 : 2,
                          ),
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    icon: Icon(
                      isRevealed ? Icons.visibility_off : Icons.visibility,
                      color: Colors.white60,
                      size: 20,
                    ),
                    onPressed: () {
                      setState(() {
                        if (isRevealed) {
                          _revealedFields.remove(key);
                        } else {
                          _revealedFields.add(key);
                        }
                      });
                    },
                  ),
                  IconButton(
                    icon: const Icon(
                      Icons.copy,
                      color: Color(0xFF00E5FF),
                      size: 20,
                    ),
                    onPressed: () => _copyToClipboard(key, val),
                  ),
                ],
              ),
            );
          }),

          // Tags section
          if (meta.tags.isNotEmpty) ...[
            const SizedBox(height: 10),
            const Text(
              'TAGS & METADATA',
              style: TextStyle(
                color: Color(0xFF00E5FF),
                fontSize: 11,
                fontWeight: FontWeight.bold,
                letterSpacing: 1.0,
              ),
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: meta.tags.map((t) {
                return Chip(
                  backgroundColor: const Color(0xFF131D31),
                  side: BorderSide(color: Colors.white.withOpacity(0.1)),
                  label: Text(
                    '#$t',
                    style: const TextStyle(color: Colors.white70, fontSize: 12),
                  ),
                );
              }).toList(),
            ),
          ],
        ],
      ),
    );
  }
}
