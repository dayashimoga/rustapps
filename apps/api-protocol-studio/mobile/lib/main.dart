import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'services/aps_service.dart';

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

  runApp(const ApsMobileApp());
}

class ApsMobileApp extends StatelessWidget {
  const ApsMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'API & Protocol Studio Mobile',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF0B132B),
        cardColor: const Color(0xFF1C2541),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF38BDF8),
          surface: Color(0xFF1C2541),
          secondary: Color(0xFF10B981),
        ),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF0F172A),
          elevation: 0,
        ),
      ),
      home: const ApsHomeScreen(),
    );
  }
}

class ApsHomeScreen extends StatefulWidget {
  const ApsHomeScreen({super.key});

  @override
  State<ApsHomeScreen> createState() => _ApsHomeScreenState();
}

class _ApsHomeScreenState extends State<ApsHomeScreen> {
  late final ApsService _service;
  int _tabIndex = 0;

  List<dynamic> _collections = [];
  List<dynamic> _requests = [];
  String? _selectedCollectionId;
  String? _selectedRequestId;
  Map<String, dynamic>? _selectedRequest;
  Map<String, dynamic>? _executionResult;
  List<dynamic> _fuzzResults = [];
  Map<String, dynamic>? _studioStats;
  bool _isLoading = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _service = ApsService();
    _loadInitialData();
  }

  void _loadInitialData() {
    setState(() => _isLoading = true);
    try {
      final stats = _service.getStudioStats();
      if (stats is Map<String, dynamic> && !stats.containsKey('error')) {
        _studioStats = stats;
      }

      final colls = _service.listCollections();
      if (colls is List && colls.isNotEmpty) {
        _collections = colls;
        _selectCollection(colls[0]['id']?.toString() ?? '');
      }
      setState(() => _isLoading = false);
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isLoading = false;
      });
    }
  }

  void _selectCollection(String colId) {
    if (colId.isEmpty) return;
    setState(() {
      _selectedCollectionId = colId;
      _isLoading = true;
    });

    try {
      final reqs = _service.listRequests(colId);
      setState(() {
        if (reqs is List) {
          _requests = reqs;
          if (reqs.isNotEmpty) {
            _selectRequest(reqs[0]['id']?.toString() ?? '');
          }
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

  void _selectRequest(String reqId) {
    if (reqId.isEmpty) return;
    setState(() => _selectedRequestId = reqId);

    try {
      final req = _service.getRequest(reqId);
      if (req is Map<String, dynamic> && !req.containsKey('error')) {
        setState(() => _selectedRequest = req);
      }
    } catch (_) {}
  }

  void _executeActiveRequest() {
    if (_selectedRequestId == null) return;
    setState(() => _isLoading = true);

    try {
      final res = _service.executeRequest(_selectedRequestId!);
      setState(() {
        if (res is Map<String, dynamic> && !res.containsKey('error')) {
          _executionResult = res;
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

  void _fuzzActiveRequest() {
    if (_selectedRequestId == null) return;
    setState(() => _isLoading = true);

    try {
      final res = _service.fuzzRequest(_selectedRequestId!, strategy: 'sql_injection');
      setState(() {
        if (res is List) {
          _fuzzResults = res;
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
            Icon(Icons.http, color: Color(0xFF38BDF8)),
            SizedBox(width: 8),
            Text(
              'API Protocol Studio',
              style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
            ),
          ],
        ),
        actions: [
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: const Color(0xFF38BDF8).withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: const Color(0xFF38BDF8).withOpacity(0.4)),
            ),
            child: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.lock_outline, size: 14, color: Color(0xFF38BDF8)),
                SizedBox(width: 4),
                Text(
                  'Air-Gapped',
                  style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: Color(0xFF38BDF8)),
                ),
              ],
            ),
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator(color: Color(0xFF38BDF8)))
          : IndexedStack(
              index: _tabIndex,
              children: [
                _buildRequestRunnerTab(),
                _buildFuzzingTab(),
                _buildTelemetryTab(),
              ],
            ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (i) => setState(() => _tabIndex = i),
        backgroundColor: const Color(0xFF0F172A),
        selectedItemColor: const Color(0xFF38BDF8),
        unselectedItemColor: const Color(0xFF64748B),
        items: const [
          BottomNavigationBarItem(icon: Icon(Icons.send_rounded), label: 'Runner'),
          BottomNavigationBarItem(icon: Icon(Icons.bug_report), label: 'Fuzzer'),
          BottomNavigationBarItem(icon: Icon(Icons.insights), label: 'Telemetry'),
        ],
      ),
    );
  }

  Widget _buildRequestRunnerTab() {
    final method = _selectedRequest?['method']?.toString() ?? 'GET';
    final url = _selectedRequest?['url']?.toString() ?? 'http://127.0.0.1:8080/health';
    final name = _selectedRequest?['name']?.toString() ?? 'Health Probe';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Collections & Requests Selectors
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: const Color(0xFF1C2541),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Target Collection & Endpoint', style: TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
              const SizedBox(height: 6),
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                    decoration: BoxDecoration(
                      color: _getMethodColor(method).withOpacity(0.2),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Text(
                      method,
                      style: TextStyle(fontWeight: FontWeight.bold, fontSize: 12, color: _getMethodColor(method)),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      name,
                      style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Text(url, style: const TextStyle(fontFamily: 'monospace', fontSize: 12, color: Color(0xFF94A3B8))),
            ],
          ),
        ),
        const SizedBox(height: 16),

        ElevatedButton.icon(
          icon: const Icon(Icons.play_arrow),
          label: const Text('Dispatch Local Loopback Request', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF38BDF8),
            foregroundColor: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _executeActiveRequest,
        ),
        const SizedBox(height: 16),

        // Execution Response Card
        if (_executionResult != null) ...[
          Card(
            color: const Color(0xFF1C2541),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        'Status: ${_executionResult?['status_code'] ?? 200}',
                        style: const TextStyle(fontWeight: FontWeight.bold, color: Color(0xFF10B981), fontSize: 14),
                      ),
                      Text(
                        '${_executionResult?['duration_ms'] ?? 12} ms',
                        style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 12),
                      ),
                    ],
                  ),
                  const Divider(height: 20, color: Color(0xFF334155)),
                  const Text('Response Body:', style: TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
                  const SizedBox(height: 6),
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: const Color(0xFF0B132B),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: SelectableText(
                      _executionResult?['body']?.toString() ?? '{"status":"ok","airgap":true}',
                      style: const TextStyle(fontFamily: 'monospace', fontSize: 11),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildFuzzingTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Offline Protocol & Mutation Fuzzer', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 8),
        const Text(
          'Inject deterministic security fuzz vectors (SQLi, command injection, buffer boundaries) directly into API contracts.',
          style: TextStyle(color: Color(0xFF94A3B8), fontSize: 12),
        ),
        const SizedBox(height: 16),
        ElevatedButton.icon(
          icon: const Icon(Icons.flash_on),
          label: const Text('Execute Offline Fuzz Mutation Suite', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFFF59E0B),
            foregroundColor: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _fuzzActiveRequest,
        ),
        const SizedBox(height: 16),
        if (_fuzzResults.isNotEmpty) ...[
          const Text('Fuzzing Findings & Assertions:', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
          const SizedBox(height: 8),
          ..._fuzzResults.map((r) {
            final map = r as Map<String, dynamic>;
            final vector = map['vector']?.toString() ?? 'payload';
            final passed = map['safe'] == true || map['blocked'] == true;

            return Card(
              color: const Color(0xFF1C2541),
              margin: const EdgeInsets.only(bottom: 8),
              child: ListTile(
                leading: Icon(passed ? Icons.check_circle : Icons.warning, color: passed ? const Color(0xFF10B981) : const Color(0xFFF43F5E)),
                title: Text(vector, style: const TextStyle(fontFamily: 'monospace', fontSize: 12)),
                subtitle: Text(passed ? 'Blocked / Handled gracefully' : 'Vulnerability detected', style: TextStyle(color: passed ? const Color(0xFF10B981) : const Color(0xFFF43F5E), fontSize: 11)),
              ),
            );
          }),
        ],
      ],
    );
  }

  Widget _buildTelemetryTab() {
    final totalRequests = _studioStats?['total_requests_executed']?.toString() ?? '1';
    final totalCollections = _studioStats?['total_collections']?.toString() ?? _collections.length.toString();
    final fuzzed = _studioStats?['total_fuzz_mutations']?.toString() ?? '24';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Studio Activity & Security Telemetry', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Collections', totalCollections, Icons.folder, const Color(0xFF38BDF8))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Executed', totalRequests, Icons.send, const Color(0xFF10B981))),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Mutations', fuzzed, Icons.bug_report, const Color(0xFFF59E0B))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Egress State', 'Air-Gapped', Icons.shield, const Color(0xFF8B5CF6))),
          ],
        ),
      ],
    );
  }

  Widget _buildMetricTile(String title, String value, IconData icon, Color color) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF1C2541),
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

  Color _getMethodColor(String method) {
    switch (method.toUpperCase()) {
      case 'GET':
        return const Color(0xFF10B981);
      case 'POST':
        return const Color(0xFF38BDF8);
      case 'PUT':
      case 'PATCH':
        return const Color(0xFFF59E0B);
      case 'DELETE':
        return const Color(0xFFF43F5E);
      default:
        return const Color(0xFF8B5CF6);
    }
  }
}
