import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/secret.dart';
import '../services/vault_service.dart';

class TotpScreen extends StatefulWidget {
  final VaultService vaultService;

  const TotpScreen({super.key, required this.vaultService});

  @override
  State<TotpScreen> createState() => _TotpScreenState();
}

class _TotpScreenState extends State<TotpScreen> {
  List<SecretMetadata> _totpSecrets = [];
  final Map<String, TotpCodeResponse> _totpCodes = {};
  Timer? _ticker;
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _loadTotpSecrets();
  }

  @override
  void dispose() {
    _ticker?.cancel();
    super.dispose();
  }

  Future<void> _loadTotpSecrets() async {
    setState(() => _isLoading = true);
    try {
      final all = await widget.vaultService.searchSecrets('*');
      final totps = all.where((s) => s.secretType == 'totp').toList();

      setState(() {
        _totpSecrets = totps;
        _isLoading = false;
      });

      await _refreshAllCodes();

      _ticker?.cancel();
      _ticker = Timer.periodic(const Duration(seconds: 1), (_) {
        _tickCountdown();
      });
    } catch (e) {
      setState(() => _isLoading = false);
    }
  }

  Future<void> _refreshAllCodes() async {
    for (final s in _totpSecrets) {
      try {
        final codeResp = await widget.vaultService.generateTotp(s.id);
        if (mounted) {
          setState(() {
            _totpCodes[s.id] = codeResp;
          });
        }
      } catch (_) {}
    }
  }

  void _tickCountdown() {
    bool needsRefresh = false;
    final updated = Map<String, TotpCodeResponse>.from(_totpCodes);

    for (final entry in updated.entries) {
      final remaining = entry.value.secondsRemaining - 1;
      if (remaining <= 0) {
        needsRefresh = true;
      } else {
        updated[entry.key] = TotpCodeResponse(
          secretId: entry.value.secretId,
          code: entry.value.code,
          secondsRemaining: remaining,
          periodSeconds: entry.value.periodSeconds,
        );
      }
    }

    if (mounted) {
      setState(() {
        _totpCodes.clear();
        _totpCodes.addAll(updated);
      });
    }

    if (needsRefresh) {
      _refreshAllCodes();
    }
  }

  void _copyTotp(String code, String title) {
    Clipboard.setData(ClipboardData(text: code));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('TOTP code for $title copied!'),
        backgroundColor: const Color(0xFF1E293B),
        behavior: SnackBarBehavior.floating,
        duration: const Duration(seconds: 2),
      ),
    );
  }

  Color _getCountdownColor(int remaining) {
    if (remaining <= 5) return const Color(0xFFFF5252);
    if (remaining <= 10) return const Color(0xFFFFB300);
    return const Color(0xFF00E5FF);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0A0E1A),
      body: _isLoading
          ? const Center(
              child: CircularProgressIndicator(color: Color(0xFF00E5FF)))
          : _totpSecrets.isEmpty
              ? Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(Icons.pin_outlined,
                          size: 56, color: Colors.white.withOpacity(0.3)),
                      const SizedBox(height: 16),
                      Text(
                        'No 2FA / TOTP Accounts',
                        style: TextStyle(
                          color: Colors.white.withOpacity(0.6),
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Text(
                        'Add a TOTP secret from the Secrets tab',
                        style: TextStyle(
                          color: Colors.white.withOpacity(0.4),
                          fontSize: 13,
                        ),
                      ),
                    ],
                  ),
                )
              : ListView.separated(
                  padding: const EdgeInsets.all(16),
                  itemCount: _totpSecrets.length,
                  separatorBuilder: (_, __) => const SizedBox(height: 14),
                  itemBuilder: (context, index) {
                    final s = _totpSecrets[index];
                    final codeResp = _totpCodes[s.id];
                    final rawCode = codeResp?.code ?? '------';
                    final formattedCode = rawCode.length == 6
                        ? '${rawCode.substring(0, 3)} ${rawCode.substring(3)}'
                        : rawCode;
                    final remaining = codeResp?.secondsRemaining ?? 30;
                    final period = codeResp?.periodSeconds ?? 30;
                    final progress = (remaining / period).clamp(0.0, 1.0);
                    final color = _getCountdownColor(remaining);

                    return Container(
                      padding: const EdgeInsets.all(18),
                      decoration: BoxDecoration(
                        color: const Color(0xFF131D31),
                        borderRadius: BorderRadius.circular(16),
                        border: Border.all(
                          color: Colors.white.withOpacity(0.08),
                        ),
                        boxShadow: [
                          BoxShadow(
                            color: Colors.black.withOpacity(0.2),
                            blurRadius: 8,
                            offset: const Offset(0, 4),
                          ),
                        ],
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text(
                                s.title,
                                style: const TextStyle(
                                  color: Colors.white,
                                  fontSize: 16,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              IconButton(
                                icon: const Icon(Icons.refresh,
                                    color: Colors.white54, size: 20),
                                onPressed: _refreshAllCodes,
                              ),
                            ],
                          ),
                          const SizedBox(height: 12),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              InkWell(
                                onTap: () => _copyTotp(rawCode, s.title),
                                child: Text(
                                  formattedCode,
                                  style: const TextStyle(
                                    color: Colors.white,
                                    fontSize: 32,
                                    fontWeight: FontWeight.bold,
                                    fontFamily: 'Courier',
                                    letterSpacing: 4,
                                  ),
                                ),
                              ),
                              Stack(
                                alignment: Alignment.center,
                                children: [
                                  SizedBox(
                                    width: 44,
                                    height: 44,
                                    child: CircularProgressIndicator(
                                      value: progress,
                                      strokeWidth: 4,
                                      backgroundColor:
                                          Colors.white.withOpacity(0.1),
                                      valueColor:
                                          AlwaysStoppedAnimation<Color>(color),
                                    ),
                                  ),
                                  Text(
                                    '$remaining',
                                    style: TextStyle(
                                      color: color,
                                      fontWeight: FontWeight.bold,
                                      fontSize: 13,
                                    ),
                                  ),
                                ],
                              ),
                            ],
                          ),
                          const SizedBox(height: 12),
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text(
                                'RFC 6238 Standard (SHA-1 / 30s)',
                                style: TextStyle(
                                  color: Colors.white.withOpacity(0.4),
                                  fontSize: 11,
                                ),
                              ),
                              TextButton.icon(
                                onPressed: () => _copyTotp(rawCode, s.title),
                                icon: const Icon(Icons.copy,
                                    size: 14, color: Color(0xFF00E5FF)),
                                label: const Text(
                                  'Copy Code',
                                  style: TextStyle(
                                    color: Color(0xFF00E5FF),
                                    fontSize: 12,
                                    fontWeight: FontWeight.bold,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    );
                  },
                ),
    );
  }
}
