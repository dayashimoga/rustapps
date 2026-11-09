import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'services/recorder_service.dart';

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
      systemNavigationBarColor: Color(0xFF0F172A),
      systemNavigationBarIconBrightness: Brightness.light,
    ),
  );

  runApp(const RecorderMobileApp());
}

class RecorderMobileApp extends StatelessWidget {
  const RecorderMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Screen Recorder & Tutorial Studio Mobile',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF090D16),
        cardColor: const Color(0xFF161F30),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFFEC4899),
          surface: Color(0xFF161F30),
          secondary: Color(0xFF38BDF8),
        ),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF0F172A),
          elevation: 0,
        ),
      ),
      home: const RecorderHomeScreen(),
    );
  }
}

class RecorderHomeScreen extends StatefulWidget {
  const RecorderHomeScreen({super.key});

  @override
  State<RecorderHomeScreen> createState() => _RecorderHomeScreenState();
}

class _RecorderHomeScreenState extends State<RecorderHomeScreen> {
  late final RecorderService _service;
  int _tabIndex = 0;

  Map<String, dynamic>? _activeSession;
  List<dynamic> _sessions = [];
  Map<String, dynamic>? _studioStats;
  bool _isLoading = false;
  String? _errorMessage;

  // Zoom & Export State
  double _zoomScale = 1.45;
  String? _exportedContent;

  @override
  void initState() {
    super.initState();
    _service = RecorderService();
    _loadSession();
  }

  void _loadSession() {
    setState(() => _isLoading = true);
    try {
      final stats = _service.getStudioStats();
      final list = _service.listRecordings();
      final current = _service.getRecording(sessionId: 'latest');

      setState(() {
        if (stats is Map<String, dynamic> && !stats.containsKey('error')) {
          _studioStats = stats;
        }
        if (list is List) {
          _sessions = list;
        }
        if (current is Map<String, dynamic> && !current.containsKey('error')) {
          _activeSession = current;
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

  void _adjustZoom() {
    final sessId = _activeSession?['session_id']?.toString() ?? 'rec-demo';
    setState(() => _isLoading = true);

    try {
      final updated = _service.adjustAutoZooms(sessId, _zoomScale);
      setState(() {
        if (updated is Map<String, dynamic> && !updated.containsKey('error')) {
          _activeSession = updated;
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

  void _exportTutorialMarkdown() {
    final sessId = _activeSession?['session_id']?.toString() ?? 'rec-demo';
    setState(() => _isLoading = true);

    try {
      final res = _service.exportTutorial(sessId, format: 'Markdown', includeCaptions: true);
      setState(() {
        if (res is Map<String, dynamic> && res.containsKey('content')) {
          _exportedContent = res['content']?.toString();
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
            Icon(Icons.videocam, color: Color(0xFFEC4899)),
            SizedBox(width: 8),
            Text(
              'Tutorial Studio',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
            ),
          ],
        ),
        actions: [
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: const Color(0xFFEC4899).withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFFEC4899).withOpacity(0.4)),
            ),
            child: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.auto_awesome, size: 14, color: Color(0xFFEC4899)),
                SizedBox(width: 4),
                Text(
                  'Auto-Zoom AI',
                  style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: Color(0xFFEC4899)),
                ),
              ],
            ),
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator(color: Color(0xFFEC4899)))
          : IndexedStack(
              index: _tabIndex,
              children: [
                _buildTimelineTab(),
                _buildAutoZoomTab(),
                _buildExportTab(),
              ],
            ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (i) => setState(() => _tabIndex = i),
        backgroundColor: const Color(0xFF0F172A),
        selectedItemColor: const Color(0xFFEC4899),
        unselectedItemColor: const Color(0xFF64748B),
        items: const [
          BottomNavigationBarItem(icon: Icon(Icons.timeline), label: 'Timeline'),
          BottomNavigationBarItem(icon: Icon(Icons.zoom_in), label: 'AI Auto-Zoom'),
          BottomNavigationBarItem(icon: Icon(Icons.file_download), label: 'Export Guide'),
        ],
      ),
    );
  }

  Widget _buildTimelineTab() {
    final title = _activeSession?['title']?.toString() ?? 'Session';
    final durationSec = ((_activeSession?['duration_ms'] as num?)?.toDouble() ?? 10000.0) / 1000.0;
    final events = (_activeSession?['events'] as List<dynamic>?) ?? [];
    final subtitles = (_activeSession?['subtitles'] as List<dynamic>?) ?? [];

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Video Preview Card
        Container(
          height: 180,
          decoration: BoxDecoration(
            color: const Color(0xFF161F30),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: const Color(0xFF334155)),
          ),
          child: Stack(
            alignment: Alignment.center,
            children: [
              const Icon(Icons.play_circle_outline, size: 64, color: Color(0xFFEC4899)),
              Positioned(
                bottom: 12,
                left: 12,
                right: 12,
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(title, style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
                    Text('${durationSec.toStringAsFixed(1)}s', style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 12)),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        Row(
          children: [
            Expanded(child: _buildMetricTile('Interactions', events.length.toString(), Icons.touch_app, const Color(0xFFEC4899))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('AI Captions', subtitles.length.toString(), Icons.subtitles, const Color(0xFF38BDF8))),
          ],
        ),
        const SizedBox(height: 20),

        const Text('Captured Interaction Events & Cues', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 14)),
        const SizedBox(height: 8),
        if (events.isEmpty)
          const Center(child: Padding(padding: EdgeInsets.symmetric(vertical: 20), child: Text('No interaction events recorded', style: TextStyle(color: Color(0xFF64748B)))))
        else
          ...events.map((e) {
            final map = e as Map<String, dynamic>;
            final type = map['event_type']?.toString() ?? 'Click';
            final tMs = map['timestamp_ms']?.toString() ?? '0';

            return Card(
              color: const Color(0xFF161F30),
              margin: const EdgeInsets.only(bottom: 6),
              child: ListTile(
                dense: true,
                leading: const Icon(Icons.mouse, size: 18, color: Color(0xFFEC4899)),
                title: Text('$type at ${tMs}ms', style: const TextStyle(fontSize: 12, fontWeight: FontWeight.bold)),
                subtitle: Text('Coordinates: (${map['x']}, ${map['y']})', style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
              ),
            );
          }),
      ],
    );
  }

  Widget _buildAutoZoomTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('AI Auto-Zoom & Focal Smoothing', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 8),
        const Text(
          'Automatically tracks user clicks and focus regions, generating cinematic bezier pan and zoom keyframes offline.',
          style: TextStyle(color: Color(0xFF94A3B8), fontSize: 12),
        ),
        const SizedBox(height: 16),
        Text('Magnification Factor: ${_zoomScale.toStringAsFixed(2)}x', style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
        Slider(
          value: _zoomScale,
          min: 1.0,
          max: 2.5,
          divisions: 30,
          label: '${_zoomScale.toStringAsFixed(2)}x',
          activeColor: const Color(0xFFEC4899),
          onChanged: (v) => setState(() => _zoomScale = v),
        ),
        const SizedBox(height: 16),
        ElevatedButton.icon(
          icon: const Icon(Icons.camera),
          label: const Text('Re-Calculate Smooth Zoom Trajectories', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFFEC4899),
            foregroundColor: Colors.white,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _adjustZoom,
        ),
      ],
    );
  }

  Widget _buildExportTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Export Step-by-Step Documentation', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 8),
        const Text('Synthesize the recorded session into clean Markdown SOP tutorials with timestamped screenshots.', style: TextStyle(color: Color(0xFF94A3B8), fontSize: 12)),
        const SizedBox(height: 16),
        ElevatedButton.icon(
          icon: const Icon(Icons.file_download),
          label: const Text('Generate Offline Markdown Tutorial SOP', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF38BDF8),
            foregroundColor: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _exportTutorialMarkdown,
        ),
        const SizedBox(height: 16),
        if (_exportedContent != null) ...[
          const Text('Synthesized SOP Tutorial:', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
          const SizedBox(height: 8),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: const Color(0xFF161F30),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFF334155)),
            ),
            child: SelectableText(
              _exportedContent!,
              style: const TextStyle(fontFamily: 'monospace', fontSize: 11),
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildMetricTile(String title, String value, IconData icon, Color color) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF161F30),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: const Color(0xFF334155)),
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
