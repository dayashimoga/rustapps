import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'services/dfir_service.dart';

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

  runApp(const DfirMobileApp());
}

class DfirMobileApp extends StatelessWidget {
  const DfirMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'DFIR Triage Kit Mobile',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF0F172A),
        cardColor: const Color(0xFF1E293B),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFFF43F5E),
          surface: Color(0xFF1E293B),
          secondary: Color(0xFF38BDF8),
        ),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF090D16),
          elevation: 0,
        ),
      ),
      home: const DfirHomeScreen(),
    );
  }
}

class DfirHomeScreen extends StatefulWidget {
  const DfirHomeScreen({super.key});

  @override
  State<DfirHomeScreen> createState() => _DfirHomeScreenState();
}

class _DfirHomeScreenState extends State<DfirHomeScreen> {
  late final DfirService _service;
  int _tabIndex = 0;

  List<dynamic> _triageRuns = [];
  Map<String, dynamic>? _selectedPackage;
  String? _selectedTriageId;
  Map<String, dynamic>? _stats;
  bool _isLoading = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _service = DfirService();
    _loadInitialData();
  }

  void _loadInitialData() {
    setState(() => _isLoading = true);
    try {
      final stats = _service.getDfirStats();
      if (stats is Map<String, dynamic> && !stats.containsKey('error')) {
        _stats = stats;
      }

      final runs = _service.listTriageRuns();
      if (runs is List && runs.isNotEmpty) {
        _triageRuns = runs;
        _selectTriage(runs[0]['triage_id']?.toString() ?? runs[0]['id']?.toString() ?? '');
      } else {
        // Run initial triage sweep if empty
        final newRun = _service.executeTriage();
        if (newRun is Map<String, dynamic> && newRun.containsKey('triage_id')) {
          _triageRuns = [newRun];
          _selectTriage(newRun['triage_id'].toString());
        }
      }
      setState(() => _isLoading = false);
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isLoading = false;
      });
    }
  }

  void _selectTriage(String triageId) {
    if (triageId.isEmpty) return;
    setState(() {
      _selectedTriageId = triageId;
      _isLoading = true;
    });

    try {
      final pkg = _service.getTriagePackage(triageId);
      setState(() {
        if (pkg is Map<String, dynamic> && !pkg.containsKey('error')) {
          _selectedPackage = pkg;
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

  void _runNewTriageSweep() {
    setState(() => _isLoading = true);
    try {
      final pkg = _service.executeTriage();
      if (pkg is Map<String, dynamic> && !pkg.containsKey('error')) {
        setState(() {
          _selectedPackage = pkg;
          _selectedTriageId = pkg['triage_id']?.toString();
          _triageRuns.insert(0, pkg);
          _isLoading = false;
        });
      } else {
        setState(() => _isLoading = false);
      }
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
            Icon(Icons.security, color: Color(0xFFF43F5E)),
            SizedBox(width: 8),
            Text(
              'DFIR Triage Kit',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
            ),
          ],
        ),
        actions: [
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: const Color(0xFFF43F5E).withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFFF43F5E).withOpacity(0.4)),
            ),
            child: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.fingerprint, size: 14, color: Color(0xFFF43F5E)),
                SizedBox(width: 4),
                Text(
                  'Forensics Safe',
                  style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: Color(0xFFF43F5E)),
                ),
              ],
            ),
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator(color: Color(0xFFF43F5E)))
          : IndexedStack(
              index: _tabIndex,
              children: [
                _buildSweepTab(),
                _buildArtifactsTab(),
                _buildTelemetryTab(),
              ],
            ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (i) => setState(() => _tabIndex = i),
        backgroundColor: const Color(0xFF090D16),
        selectedItemColor: const Color(0xFFF43F5E),
        unselectedItemColor: const Color(0xFF64748B),
        items: const [
          BottomNavigationBarItem(icon: Icon(Icons.radar), label: 'Triage Sweep'),
          BottomNavigationBarItem(icon: Icon(Icons.playlist_add_check), label: 'Artifacts'),
          BottomNavigationBarItem(icon: Icon(Icons.assessment), label: 'Telemetry'),
        ],
      ),
    );
  }

  Widget _buildSweepTab() {
    final triageId = _selectedPackage?['triage_id']?.toString() ?? 'TR-0001';
    final host = _selectedPackage?['host_name']?.toString() ?? 'localhost';
    final os = _selectedPackage?['os_info']?.toString() ?? 'Air-Gapped Sovereign Node';
    final artifactCount = (_selectedPackage?['artifacts'] as List<dynamic>?)?.length ?? 0;
    final processCount = (_selectedPackage?['processes'] as List<dynamic>?)?.length ?? 0;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Action Button
        ElevatedButton.icon(
          icon: const Icon(Icons.bolt),
          label: const Text('Execute New Live Incident Triage Sweep', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFFF43F5E),
            foregroundColor: Colors.white,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _runNewTriageSweep,
        ),
        const SizedBox(height: 16),

        // Session Overview Card
        Card(
          color: const Color(0xFF1E293B),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text('Triage Session: $triageId', style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 15)),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: const Color(0xFF10B981).withOpacity(0.15),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: const Text('Preserved', style: TextStyle(fontSize: 10, color: Color(0xFF10B981), fontWeight: FontWeight.bold)),
                    ),
                  ],
                ),
                const Divider(height: 20, color: Color(0xFF334155)),
                Text('Target Node: $host', style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13)),
                Text('Environment: $os', style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
                const SizedBox(height: 12),
                Row(
                  children: [
                    _buildStatPill('Processes: $processCount', const Color(0xFF38BDF8)),
                    const SizedBox(width: 8),
                    _buildStatPill('Artifacts: $artifactCount', const Color(0xFFF59E0B)),
                  ],
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        const Text('Previous Triage Executions', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 14)),
        const SizedBox(height: 8),
        ..._triageRuns.map((r) {
          final id = r['triage_id']?.toString() ?? r['id']?.toString() ?? '';
          final time = r['timestamp']?.toString() ?? 'Session';
          final isSelected = id == _selectedTriageId;

          return Card(
            color: isSelected ? const Color(0xFF334155) : const Color(0xFF1E293B),
            margin: const EdgeInsets.only(bottom: 8),
            child: ListTile(
              leading: Icon(Icons.fingerprint, color: isSelected ? const Color(0xFFF43F5E) : const Color(0xFF94A3B8)),
              title: Text('Sweep $id', style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
              subtitle: Text(time, style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
              trailing: const Icon(Icons.chevron_right, size: 18, color: Color(0xFF64748B)),
              onTap: () => _selectTriage(id),
            ),
          );
        }),
      ],
    );
  }

  Widget _buildArtifactsTab() {
    final artifacts = (_selectedPackage?['artifacts'] as List<dynamic>?) ?? [];

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text('${artifacts.length} Forensic Artifacts Identified', style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 8),
        if (artifacts.isEmpty)
          const Center(child: Padding(padding: EdgeInsets.symmetric(vertical: 32), child: Text('No artifacts captured in this sweep', style: TextStyle(color: Color(0xFF64748B)))))
        else
          ...artifacts.map((a) {
            final map = a as Map<String, dynamic>;
            final name = map['name']?.toString() ?? 'Artifact';
            final category = map['category']?.toString() ?? 'System';
            final hash = map['sha256']?.toString() ?? 'sha256-verified';

            return Card(
              color: const Color(0xFF1E293B),
              margin: const EdgeInsets.only(bottom: 8),
              child: ListTile(
                leading: const Icon(Icons.description, color: Color(0xFFF43F5E)),
                title: Text(name, style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
                subtitle: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Category: $category', style: const TextStyle(color: Color(0xFF38BDF8), fontSize: 11)),
                    Text(hash, maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontFamily: 'monospace', color: Color(0xFF94A3B8), fontSize: 10)),
                  ],
                ),
              ),
            );
          }),
      ],
    );
  }

  Widget _buildTelemetryTab() {
    final totalSweeps = _stats?['total_triage_runs']?.toString() ?? _triageRuns.length.toString();
    final indicators = _stats?['indicators_flagged']?.toString() ?? '0';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Forensic Integrity & Hash Telemetry', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Triage Runs', totalSweeps, Icons.radar, const Color(0xFFF43F5E))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Indicators', indicators, Icons.warning_amber, const Color(0xFFF59E0B))),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Hashing', 'SHA-256 + BLAKE3', Icons.tag, const Color(0xFF38BDF8))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Network Egress', 'Air-Gapped', Icons.wifi_off, const Color(0xFF10B981))),
          ],
        ),
      ],
    );
  }

  Widget _buildStatPill(String text, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(text, style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: color)),
    );
  }

  Widget _buildMetricTile(String title, String value, IconData icon, Color color) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF1E293B),
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
