import 'dart:convert';
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
        cardColor: const Color(0xFF131D2E),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF06B6D4),
          surface: Color(0xFF131D2E),
          secondary: Color(0xFF10B981),
          error: Color(0xFFF43F5E),
        ),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF131D2E),
          elevation: 0,
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
  late final QrStudioService _service;
  int _tabIndex = 0;

  // Generator State
  final _payloadController = TextEditingController(text: 'https://rustapps.offline');
  final _labelController = TextEditingController(text: 'Official Offline URL');
  final _passphraseController = TextEditingController();
  String _selectedEcc = 'Medium';
  String _selectedShape = 'Rounded';
  String _selectedFinder = 'Rounded';
  bool _signEd25519 = false;
  bool _encryptPayload = false;
  Map<String, dynamic>? _lastGeneratedResult;
  String? _generateError;
  bool _isGenerating = false;

  // Scanner State
  final _scanInputController = TextEditingController();
  Map<String, dynamic>? _scanResult;
  String? _scanError;
  bool _isScanning = false;

  // History & Stats State
  Map<String, dynamic>? _studioStats;
  List<dynamic> _historyItems = [];
  bool _isLoadingHistory = false;

  @override
  void initState() {
    super.initState();
    _service = QrStudioService();
    _loadStatsAndHistory();
  }

  void _loadStatsAndHistory() {
    setState(() => _isLoadingHistory = true);
    try {
      final stats = _service.getStudioStats();
      final history = _service.listHistory(limit: 50);
      setState(() {
        if (!stats.containsKey('error')) {
          _studioStats = stats;
        }
        if (history is List) {
          _historyItems = history;
        } else if (history.containsKey('items') && history['items'] is List) {
          _historyItems = history['items'] as List;
        }
        _isLoadingHistory = false;
      });
    } catch (_) {
      setState(() => _isLoadingHistory = false);
    }
  }

  void _generateQrCode() {
    final text = _payloadController.text.trim();
    if (text.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Please enter payload text or URL')),
      );
      return;
    }

    setState(() {
      _isGenerating = true;
      _generateError = null;
    });

    final res = _service.generateQr(
      rawText: text,
      label: _labelController.text.trim(),
      eccLevel: _selectedEcc,
      moduleShape: _selectedShape,
      finderStyle: _selectedFinder,
      passphrase: _encryptPayload ? _passphraseController.text.trim() : null,
      signEd25519: _signEd25519,
    );

    setState(() {
      _isGenerating = false;
      if (res.containsKey('error')) {
        _generateError = res['error'].toString();
        _lastGeneratedResult = null;
      } else {
        _lastGeneratedResult = res;
        _generateError = null;
      }
    });
  }

  void _analyzeOrScanPayload() {
    final input = _scanInputController.text.trim();
    if (input.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Please enter or paste QR payload / base64')),
      );
      return;
    }

    setState(() {
      _isScanning = true;
      _scanError = null;
    });

    // Check if input looks like base64 image or raw payload
    Map<String, dynamic> res;
    if (input.startsWith('data:image') || input.length > 500) {
      res = _service.scanQrImage(pngBase64: input);
    } else {
      res = _service.analyzeSecurity(input);
    }

    setState(() {
      _isScanning = false;
      if (res.containsKey('error')) {
        _scanError = res['error'].toString();
        _scanResult = null;
      } else {
        _scanResult = res;
        _scanError = null;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Row(
          children: [
            Icon(Icons.qr_code_2, color: Color(0xFF06B6D4)),
            SizedBox(width: 8),
            Text(
              'QR Code Studio',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
            ),
          ],
        ),
        actions: [
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: const Color(0xFF06B6D4).withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFF06B6D4).withOpacity(0.4)),
            ),
            child: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.wifi_off, size: 14, color: Color(0xFF06B6D4)),
                SizedBox(width: 4),
                Text(
                  '100% Offline',
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.bold,
                    color: Color(0xFF06B6D4),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
      body: IndexedStack(
        index: _tabIndex,
        children: [
          _buildGeneratorTab(),
          _buildScannerTab(),
          _buildHistoryTab(),
        ],
      ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (i) {
          setState(() => _tabIndex = i);
          if (i == 2) _loadStatsAndHistory();
        },
        backgroundColor: const Color(0xFF131D2E),
        selectedItemColor: const Color(0xFF06B6D4),
        unselectedItemColor: const Color(0xFF64748B),
        items: const [
          BottomNavigationBarItem(
            icon: Icon(Icons.qr_code_scanner),
            label: 'Generator',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.shield_outlined),
            label: 'Anti-Quishing',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.query_stats),
            label: 'Audit & Stats',
          ),
        ],
      ),
    );
  }

  Widget _buildGeneratorTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Presets selector
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            children: [
              _buildPresetChip('URL', 'https://rustapps.offline', Icons.link),
              const SizedBox(width: 8),
              _buildPresetChip('Wi-Fi', 'WIFI:T:WPA;S:AirGappedLab;P:TopSecretKey123;;', Icons.wifi),
              const SizedBox(width: 8),
              _buildPresetChip('Text', 'Offline Sovereign Workstation v1.0', Icons.text_snippet),
              const SizedBox(width: 8),
              _buildPresetChip('Contact', 'BEGIN:VCARD\nVERSION:3.0\nFN:Dayan S\nEMAIL:dayan@rustapps.offline\nEND:VCARD', Icons.person),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // Text inputs
        TextField(
          controller: _payloadController,
          maxLines: 2,
          decoration: InputDecoration(
            labelText: 'QR Payload / Text / URL',
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
            filled: true,
            fillColor: const Color(0xFF131D2E),
            prefixIcon: const Icon(Icons.edit, color: Color(0xFF06B6D4)),
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _labelController,
          decoration: InputDecoration(
            labelText: 'Label (Optional)',
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
            filled: true,
            fillColor: const Color(0xFF131D2E),
          ),
        ),
        const SizedBox(height: 16),

        // Geometry & Style Grid
        Card(
          color: const Color(0xFF131D2E),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'Geometry & Error Correction',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13, color: Color(0xFF94A3B8)),
                ),
                const SizedBox(height: 10),
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        value: _selectedEcc,
                        decoration: const InputDecoration(labelText: 'ECC Level', border: OutlineInputBorder()),
                        items: const [
                          DropdownMenuItem(value: 'Low', child: Text('Low (7%)')),
                          DropdownMenuItem(value: 'Medium', child: Text('Medium (15%)')),
                          DropdownMenuItem(value: 'Quartile', child: Text('Quartile (25%)')),
                          DropdownMenuItem(value: 'High', child: Text('High (30%)')),
                        ],
                        onChanged: (v) => setState(() => _selectedEcc = v ?? 'Medium'),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        value: _selectedShape,
                        decoration: const InputDecoration(labelText: 'Module Shape', border: OutlineInputBorder()),
                        items: const [
                          DropdownMenuItem(value: 'Square', child: Text('Square')),
                          DropdownMenuItem(value: 'Rounded', child: Text('Rounded')),
                          DropdownMenuItem(value: 'Dots', child: Text('Dots')),
                          DropdownMenuItem(value: 'Diamond', child: Text('Diamond')),
                        ],
                        onChanged: (v) => setState(() => _selectedShape = v ?? 'Rounded'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        value: _selectedFinder,
                        decoration: const InputDecoration(labelText: 'Finder Pattern', border: OutlineInputBorder()),
                        items: const [
                          DropdownMenuItem(value: 'Square', child: Text('Square Eyes')),
                          DropdownMenuItem(value: 'Rounded', child: Text('Rounded Eyes')),
                          DropdownMenuItem(value: 'Circle', child: Text('Circle Eyes')),
                        ],
                        onChanged: (v) => setState(() => _selectedFinder = v ?? 'Rounded'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                SwitchListTile(
                  title: const Text('Ed25519 Cryptographic Signature', style: TextStyle(fontSize: 13)),
                  subtitle: const Text('Attach cryptographic signature verifying authenticity', style: TextStyle(fontSize: 11)),
                  value: _signEd25519,
                  activeColor: const Color(0xFF06B6D4),
                  onChanged: (val) => setState(() => _signEd25519 = val),
                  contentPadding: EdgeInsets.zero,
                ),
                SwitchListTile(
                  title: const Text('AES-256-GCM Encrypted Envelope', style: TextStyle(fontSize: 13)),
                  subtitle: const Text('Encrypt payload bytes with zero-knowledge passphrase', style: TextStyle(fontSize: 11)),
                  value: _encryptPayload,
                  activeColor: const Color(0xFF10B981),
                  onChanged: (val) => setState(() => _encryptPayload = val),
                  contentPadding: EdgeInsets.zero,
                ),
                if (_encryptPayload) ...[
                  const SizedBox(height: 8),
                  TextField(
                    controller: _passphraseController,
                    obscureText: true,
                    decoration: InputDecoration(
                      labelText: 'Passphrase for AES-256-GCM',
                      border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
                      filled: true,
                      fillColor: const Color(0xFF0A0E17),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        // Generate Action Button
        ElevatedButton.icon(
          icon: _isGenerating
              ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.black))
              : const Icon(Icons.bolt),
          label: Text(
            _isGenerating ? 'Synthesizing...' : 'Generate ISO/IEC 18004 QR Matrix',
            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 15),
          ),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF06B6D4),
            foregroundColor: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _isGenerating ? null : _generateQrCode,
        ),
        const SizedBox(height: 16),

        // Error display
        if (_generateError != null) ...[
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: const Color(0xFFF43F5E).withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFFF43F5E)),
            ),
            child: Row(
              children: [
                const Icon(Icons.error_outline, color: Color(0xFFF43F5E)),
                const SizedBox(width: 8),
                Expanded(child: Text(_generateError!, style: const TextStyle(color: Color(0xFFF43F5E), fontSize: 12))),
              ],
            ),
          ),
          const SizedBox(height: 16),
        ],

        // Rendered QR Output
        if (_lastGeneratedResult != null) _buildResultDisplayCard(_lastGeneratedResult!),
      ],
    );
  }

  Widget _buildPresetChip(String label, String payload, IconData icon) {
    return ActionChip(
      avatar: Icon(icon, size: 16, color: const Color(0xFF06B6D4)),
      label: Text(label, style: const TextStyle(fontSize: 12)),
      backgroundColor: const Color(0xFF131D2E),
      onPressed: () {
        setState(() {
          _payloadController.text = payload;
          _labelController.text = 'Preset $label';
        });
      },
    );
  }

  Widget _buildResultDisplayCard(Map<String, dynamic> res) {
    final pngBase64 = res['png_base64'] as String?;
    final version = res['version'] ?? 1;
    final dim = res['dimension'] ?? 21;
    final ecc = res['ecc_level'] ?? 'Medium';
    final contrast = res['contrast_ratio'] != null ? (res['contrast_ratio'] as num).toStringAsFixed(1) : '21.0';
    final securityReport = res['security_report'] as Map<String, dynamic>?;
    final riskLevel = securityReport?['risk_level']?.toString() ?? 'Safe';

    return Card(
      color: const Color(0xFF131D2E),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            // QR Image
            if (pngBase64 != null && pngBase64.isNotEmpty) ...[
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Image.memory(
                  base64Decode(pngBase64),
                  width: 220,
                  height: 220,
                  fit: BoxFit.contain,
                ),
              ),
              const SizedBox(height: 12),
            ],

            // Badges row
            Wrap(
              spacing: 8,
              runSpacing: 6,
              alignment: WrapAlignment.center,
              children: [
                _buildBadge('v$version ($dim×$dim)', const Color(0xFF06B6D4)),
                _buildBadge('ECC $ecc', const Color(0xFF8B5CF6)),
                _buildBadge('Contrast $contrast:1', const Color(0xFF10B981)),
                _buildBadge('Risk: $riskLevel', _getRiskColor(riskLevel)),
              ],
            ),
            const SizedBox(height: 12),

            // Metadata text
            SelectableText(
              res['encoded_payload']?.toString() ?? res['raw_payload']?.toString() ?? '',
              style: const TextStyle(fontFamily: 'monospace', fontSize: 11, color: Color(0xFF94A3B8)),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildScannerTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text(
          'Air-Gapped Anti-Quishing & Payload Inspector',
          style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16),
        ),
        const SizedBox(height: 8),
        const Text(
          'Inspect raw payloads, encoded QR tokens, or base64 images for phishing heuristics, deceptive unicode homoglyphs, and unauthorized redirects without touching any network socket.',
          style: TextStyle(color: Color(0xFF94A3B8), fontSize: 12),
        ),
        const SizedBox(height: 16),
        TextField(
          controller: _scanInputController,
          maxLines: 4,
          decoration: InputDecoration(
            labelText: 'Paste QR Text, URL, or Base64 PNG bytes',
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
            filled: true,
            fillColor: const Color(0xFF131D2E),
          ),
        ),
        const SizedBox(height: 12),
        ElevatedButton.icon(
          icon: _isScanning
              ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.black))
              : const Icon(Icons.security),
          label: const Text('Inspect Security & Decrypt', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF10B981),
            foregroundColor: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _isScanning ? null : _analyzeOrScanPayload,
        ),
        const SizedBox(height: 16),

        if (_scanError != null) ...[
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: const Color(0xFFF43F5E).withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFFF43F5E)),
            ),
            child: Text(_scanError!, style: const TextStyle(color: Color(0xFFF43F5E), fontSize: 12)),
          ),
        ],

        if (_scanResult != null) _buildScanResultCard(_scanResult!),
      ],
    );
  }

  Widget _buildScanResultCard(Map<String, dynamic> res) {
    final riskLevel = res['risk_level']?.toString() ?? 'Safe';
    final riskScore = res['risk_score']?.toString() ?? '0';
    final payloadType = res['payload_type']?.toString() ?? 'Text';
    final hostDomain = res['host_domain']?.toString();
    final findings = res['findings'] as List<dynamic>? ?? [];

    return Card(
      color: const Color(0xFF131D2E),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.shield, color: _getRiskColor(riskLevel)),
                const SizedBox(width: 8),
                Text(
                  'Risk Assessment: $riskLevel ($riskScore/100)',
                  style: TextStyle(fontWeight: FontWeight.bold, color: _getRiskColor(riskLevel), fontSize: 15),
                ),
              ],
            ),
            const Divider(height: 24, color: Color(0xFF1E293B)),
            Text('Payload Classification: $payloadType', style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13)),
            if (hostDomain != null) ...[
              const SizedBox(height: 4),
              Text('Host Domain: $hostDomain', style: const TextStyle(color: Color(0xFF06B6D4), fontSize: 12)),
            ],
            const SizedBox(height: 12),
            const Text('Heuristic Findings:', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 12, color: Color(0xFF94A3B8))),
            const SizedBox(height: 6),
            if (findings.isEmpty)
              const Text('No security risks or homoglyph deceits detected.', style: TextStyle(color: Color(0xFF10B981), fontSize: 12))
            else
              ...findings.map((f) {
                final map = f as Map<String, dynamic>;
                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Icon(Icons.warning_amber_rounded, size: 16, color: Color(0xFFF59E0B)),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(map['title']?.toString() ?? '', style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 12)),
                            Text(map['description']?.toString() ?? '', style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
                          ],
                        ),
                      ),
                    ],
                  ),
                );
              }),
          ],
        ),
      ),
    );
  }

  Widget _buildHistoryTab() {
    if (_isLoadingHistory) {
      return const Center(child: CircularProgressIndicator(color: Color(0xFF06B6D4)));
    }

    final totalGenerated = _studioStats?['total_generated']?.toString() ?? '0';
    final totalScanned = _studioStats?['total_scanned']?.toString() ?? '0';
    final safePayloads = _studioStats?['safe_payloads']?.toString() ?? '0';
    final threatsBlocked = _studioStats?['threats_blocked']?.toString() ?? '0';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Studio Telemetry & Audit Log', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 12),

        // 4 Stats Cards
        Row(
          children: [
            Expanded(child: _buildMetricTile('Generated', totalGenerated, Icons.qr_code, const Color(0xFF06B6D4))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Scanned', totalScanned, Icons.document_scanner, const Color(0xFF8B5CF6))),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Safe', safePayloads, Icons.check_circle_outline, const Color(0xFF10B981))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Threats Blocked', threatsBlocked, Icons.block, const Color(0xFFF43F5E))),
          ],
        ),
        const SizedBox(height: 20),

        const Text('Audit History', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 14)),
        const SizedBox(height: 8),

        if (_historyItems.isEmpty)
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 24),
            child: Center(
              child: Text('No recorded QR transactions yet.', style: TextStyle(color: Color(0xFF64748B))),
            ),
          )
        else
          ..._historyItems.map((item) {
            final m = item as Map<String, dynamic>;
            final label = m['label']?.toString() ?? 'Untitled QR';
            final raw = m['raw_payload']?.toString() ?? '';
            final pType = m['payload_type']?.toString() ?? 'Text';

            return Card(
              color: const Color(0xFF131D2E),
              margin: const EdgeInsets.only(bottom: 8),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
              child: ListTile(
                leading: const Icon(Icons.qr_code, color: Color(0xFF06B6D4)),
                title: Text(label, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13)),
                subtitle: Text(
                  '$pType • $raw',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11),
                ),
                trailing: const Icon(Icons.chevron_right, size: 18, color: Color(0xFF64748B)),
              ),
            );
          }),
      ],
    );
  }

  Widget _buildMetricTile(String title, String value, IconData icon, Color color) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF131D2E),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: const Color(0xFF1E293B)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(title, style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
              Icon(icon, size: 16, color: color),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            value,
            style: TextStyle(fontWeight: FontWeight.bold, fontSize: 20, color: color),
          ),
        ],
      ),
    );
  }

  Widget _buildBadge(String label, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: color.withOpacity(0.5)),
      ),
      child: Text(
        label,
        style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: color),
      ),
    );
  }

  Color _getRiskColor(String level) {
    switch (level.toUpperCase()) {
      case 'SAFE':
        return const Color(0xFF10B981);
      case 'LOWRISK':
      case 'LOW_RISK':
        return const Color(0xFF38BDF8);
      case 'MEDIUMRISK':
      case 'MEDIUM_RISK':
        return const Color(0xFFF59E0B);
      case 'HIGHRISK':
      case 'HIGH_RISK':
      case 'CRITICAL':
        return const Color(0xFFF43F5E);
      default:
        return const Color(0xFF10B981);
    }
  }
}
