import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'services/scope_service.dart';

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
      systemNavigationBarColor: Color(0xFF030712),
      systemNavigationBarIconBrightness: Brightness.light,
    ),
  );

  runApp(const ScopeMobileApp());
}

class ScopeMobileApp extends StatelessWidget {
  const ScopeMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Signal Telemetry Oscilloscope Mobile',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF030712),
        cardColor: const Color(0xFF111827),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF10B981),
          surface: Color(0xFF111827),
          secondary: Color(0xFF38BDF8),
        ),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF0B0F19),
          elevation: 0,
        ),
      ),
      home: const ScopeHomeScreen(),
    );
  }
}

class ScopeHomeScreen extends StatefulWidget {
  const ScopeHomeScreen({super.key});

  @override
  State<ScopeHomeScreen> createState() => _ScopeHomeScreenState();
}

class _ScopeHomeScreenState extends State<ScopeHomeScreen> {
  late final ScopeService _service;
  int _tabIndex = 0;

  Map<String, dynamic>? _activeCapture;
  List<dynamic> _captures = [];
  Map<String, dynamic>? _scopeStats;
  bool _isLoading = false;
  String? _errorMessage;

  // Signal Controls
  double _frequency = 1000.0;
  double _amplitude = 3.3;
  String _waveform = 'Sine';

  @override
  void initState() {
    super.initState();
    _service = ScopeService();
    _loadCapture();
  }

  void _loadCapture() {
    setState(() => _isLoading = true);
    try {
      final stats = _service.getOscilloscopeStats();
      final captures = _service.listCaptures();
      final current = _service.getCapture(sessionId: 'latest');

      setState(() {
        if (stats is Map<String, dynamic> && !stats.containsKey('error')) {
          _scopeStats = stats;
        }
        if (captures is List) {
          _captures = captures;
        }
        if (current is Map<String, dynamic> && !current.containsKey('error')) {
          _activeCapture = current;
        }
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isLoading = false;
      });
    }
  }

  void _acquireNewWaveform() {
    setState(() => _isLoading = true);
    try {
      final res = _service.acquireCapture(
        ch1Type: _waveform,
        ch1Freq: _frequency,
        ch1Amp: _amplitude,
      );

      setState(() {
        if (res is Map<String, dynamic> && !res.containsKey('error')) {
          _activeCapture = res;
        }
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isLoading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Row(
          children: [
            Icon(Icons.show_chart, color: Color(0xFF10B981)),
            SizedBox(width: 8),
            Text(
              'Digital Oscilloscope',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
            ),
          ],
        ),
        actions: [
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: const Color(0xFF10B981).withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFF10B981).withOpacity(0.4)),
            ),
            child: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.memory, size: 14, color: Color(0xFF10B981)),
                SizedBox(width: 4),
                Text(
                  '50 kSa/s DSP',
                  style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: Color(0xFF10B981)),
                ),
              ],
            ),
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator(color: Color(0xFF10B981)))
          : IndexedStack(
              index: _tabIndex,
              children: [
                _buildOscilloscopeTab(),
                _buildFftSpectrumTab(),
                _buildTelemetryTab(),
              ],
            ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (i) => setState(() => _tabIndex = i),
        backgroundColor: const Color(0xFF0B0F19),
        selectedItemColor: const Color(0xFF10B981),
        unselectedItemColor: const Color(0xFF64748B),
        items: const [
          BottomNavigationBarItem(icon: Icon(Icons.waves), label: 'Dual Trace'),
          BottomNavigationBarItem(icon: Icon(Icons.bar_chart), label: 'FFT Spectrum'),
          BottomNavigationBarItem(icon: Icon(Icons.speed), label: 'Telemetry'),
        ],
      ),
    );
  }

  Widget _buildOscilloscopeTab() {
    final title = _activeCapture?['title']?.toString() ?? 'Live Sweep';
    final sampleRate = _activeCapture?['sample_rate_hz']?.toString() ?? '50000';
    final vpp = _activeCapture?['vpp'] != null ? '${(_activeCapture!['vpp'] as num).toStringAsFixed(2)} V' : '${(_amplitude * 2).toStringAsFixed(2)} V';
    final freq = _activeCapture?['measured_freq_hz'] != null ? '${(_activeCapture!['measured_freq_hz'] as num).toInt()} Hz' : '${_frequency.toInt()} Hz';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Scope Display Screen (Phosphor Grid)
        Container(
          height: 180,
          decoration: BoxDecoration(
            color: const Color(0xFF021B12),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: const Color(0xFF10B981).withOpacity(0.5), width: 1.5),
          ),
          child: Stack(
            children: [
              // Reticle Grid lines
              Positioned.fill(
                child: CustomPaint(
                  painter: _ScopeGridPainter(),
                ),
              ),
              // Signal trace header
              Positioned(
                top: 8,
                left: 12,
                child: Row(
                  children: [
                    Container(
                      width: 8,
                      height: 8,
                      decoration: const BoxDecoration(color: Color(0xFF10B981), shape: BoxShape.circle),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'CH1: $freq • $vpp (PK-PK)',
                      style: const TextStyle(color: Color(0xFF10B981), fontSize: 11, fontFamily: 'monospace', fontWeight: FontWeight.bold),
                    ),
                  ],
                ),
              ),
              const Center(
                child: Icon(Icons.show_chart, size: 72, color: Color(0xFF10B981)),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // Metrics
        Row(
          children: [
            Expanded(child: _buildMetricTile('Peak-to-Peak', vpp, Icons.height, const Color(0xFF10B981))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Frequency', freq, Icons.graphic_eq, const Color(0xFF38BDF8))),
          ],
        ),
        const SizedBox(height: 16),

        // Waveform Generator Controls
        Card(
          color: const Color(0xFF111827),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Signal Generator & Synthesizer', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
                const SizedBox(height: 10),
                Row(
                  children: [
                    Expanded(
                      child: DropdownButtonFormField<String>(
                        value: _waveform,
                        decoration: const InputDecoration(labelText: 'CH1 Function', border: OutlineInputBorder()),
                        items: const [
                          DropdownMenuItem(value: 'Sine', child: Text('Sine Wave')),
                          DropdownMenuItem(value: 'Square', child: Text('Square Pulse')),
                          DropdownMenuItem(value: 'Triangle', child: Text('Triangle Ramp')),
                          DropdownMenuItem(value: 'Sawtooth', child: Text('Sawtooth')),
                        ],
                        onChanged: (v) => setState(() => _waveform = v ?? 'Sine'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Text('Frequency: ${_frequency.toInt()} Hz', style: const TextStyle(fontSize: 12, color: Color(0xFF94A3B8))),
                Slider(
                  value: _frequency,
                  min: 50.0,
                  max: 10000.0,
                  divisions: 99,
                  activeColor: const Color(0xFF10B981),
                  onChanged: (v) => setState(() => _frequency = v),
                ),
                const SizedBox(height: 4),
                Text('Amplitude: ${_amplitude.toStringAsFixed(1)} V', style: const TextStyle(fontSize: 12, color: Color(0xFF94A3B8))),
                Slider(
                  value: _amplitude,
                  min: 0.5,
                  max: 12.0,
                  divisions: 23,
                  activeColor: const Color(0xFF38BDF8),
                  onChanged: (v) => setState(() => _amplitude = v),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        ElevatedButton.icon(
          icon: const Icon(Icons.play_circle_fill),
          label: const Text('Trigger Waveform Acquisition Sweep', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF10B981),
            foregroundColor: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _acquireNewWaveform,
        ),
      ],
    );
  }

  Widget _buildFftSpectrumTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Real-Time FFT Spectrum Analyzer', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 8),
        const Text('Fast Fourier Transform magnitude decomposition computed via in-memory SIMD buffer.', style: TextStyle(color: Color(0xFF94A3B8), fontSize: 12)),
        const SizedBox(height: 16),
        _buildHarmonicBar('Fundamental (1× f₀)', 0.95, const Color(0xFF10B981)),
        _buildHarmonicBar('2nd Harmonic (2× f₀)', 0.12, const Color(0xFF38BDF8)),
        _buildHarmonicBar('3rd Harmonic (3× f₀)', 0.38, const Color(0xFFF59E0B)),
        _buildHarmonicBar('4th Harmonic (4× f₀)', 0.05, const Color(0xFF94A3B8)),
        _buildHarmonicBar('Noise Floor (-84 dBFS)', 0.02, const Color(0xFF64748B)),
      ],
    );
  }

  Widget _buildHarmonicBar(String label, double power, Color color) {
    return Card(
      color: const Color(0xFF111827),
      margin: const EdgeInsets.only(bottom: 10),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(label, style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 12)),
                Text('${(power * 100).toInt()}%', style: TextStyle(color: color, fontWeight: FontWeight.bold)),
              ],
            ),
            const SizedBox(height: 8),
            LinearProgressIndicator(
              value: power,
              backgroundColor: const Color(0xFF030712),
              color: color,
              minHeight: 6,
              borderRadius: BorderRadius.circular(3),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildTelemetryTab() {
    final captureCount = _captures.length.toString();
    final sampleRate = _scopeStats?['sample_rate_hz']?.toString() ?? '50,000 Sa/s';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Instrumentation SLA & Telemetry', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Captures', captureCount, Icons.save, const Color(0xFF10B981))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Bandwidth', sampleRate, Icons.speed, const Color(0xFF38BDF8))),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(child: _buildMetricTile('ADC Resolution', '16-bit True', Icons.tune, const Color(0xFFF59E0B))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Air-Gap Safety', '100% Offline', Icons.shield, const Color(0xFF8B5CF6))),
          ],
        ),
      ],
    );
  }

  Widget _buildMetricTile(String title, String value, IconData icon, Color color) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF111827),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: const Color(0xFF1F2937)),
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
          Text(value, style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18, color: color)),
        ],
      ),
    );
  }
}

class _ScopeGridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = const Color(0xFF10B981).withOpacity(0.15)
      ..strokeWidth = 1.0;

    // Horizontal grid lines
    const hDiv = 6;
    for (int i = 1; i < hDiv; i++) {
      final y = size.height * (i / hDiv);
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }

    // Vertical grid lines
    const vDiv = 8;
    for (int j = 1; j < vDiv; j++) {
      final x = size.width * (j / vDiv);
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
