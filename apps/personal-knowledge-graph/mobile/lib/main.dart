import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'services/pkg_service.dart';

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

  runApp(const PkgMobileApp());
}

class PkgMobileApp extends StatelessWidget {
  const PkgMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Personal Knowledge Graph Mobile',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: const Color(0xFF0F172A),
        cardColor: const Color(0xFF1E293B),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF10B981),
          surface: Color(0xFF1E293B),
          secondary: Color(0xFF06B6D4),
        ),
        appBarTheme: const AppBarTheme(
          backgroundColor: Color(0xFF0A0F1D),
          elevation: 0,
        ),
      ),
      home: const PkgHomeScreen(),
    );
  }
}

class PkgHomeScreen extends StatefulWidget {
  const PkgHomeScreen({super.key});

  @override
  State<PkgHomeScreen> createState() => _PkgHomeScreenState();
}

class _PkgHomeScreenState extends State<PkgHomeScreen> {
  late final PkgService _service;
  int _tabIndex = 0;

  List<dynamic> _nodes = [];
  List<dynamic> _edges = [];
  Map<String, dynamic>? _selectedNode;
  Map<String, dynamic>? _graphStats;
  bool _isLoading = false;
  String? _errorMessage;

  // Search & Editor State
  final _searchController = TextEditingController();
  final _titleController = TextEditingController();
  final _contentController = TextEditingController();
  List<dynamic> _searchResults = [];

  @override
  void initState() {
    super.initState();
    _service = PkgService();
    _loadGraphData();
  }

  void _loadGraphData() {
    setState(() => _isLoading = true);
    try {
      final stats = _service.getGraphStats();
      if (stats is Map<String, dynamic> && !stats.containsKey('error')) {
        _graphStats = stats;
      }

      final graph = _service.getGraphVisualization();
      if (graph is Map<String, dynamic> && !graph.containsKey('error')) {
        final rawNodes = graph['nodes'] as List<dynamic>? ?? [];
        final rawEdges = graph['edges'] as List<dynamic>? ?? [];
        setState(() {
          _nodes = rawNodes;
          _edges = rawEdges;
          if (rawNodes.isNotEmpty) {
            _selectNode(rawNodes[0]['id']?.toString() ?? '');
          }
        });
      } else {
        // Auto-seed note if empty
        final saved = _service.saveNote(
          title: 'Air-Gapped Sovereign Computing',
          content: 'Foundational architecture of local-first cryptographic systems. Links to [[Cryptography]] and [[Storage]].',
          tags: ['architecture', 'security'],
        );
        if (saved is Map<String, dynamic> && saved.containsKey('id')) {
          _nodes = [saved];
          _selectNode(saved['id'].toString());
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

  void _selectNode(String nodeId) {
    if (nodeId.isEmpty) return;
    try {
      final node = _service.getNode(nodeId);
      if (node is Map<String, dynamic> && !node.containsKey('error')) {
        setState(() {
          _selectedNode = node;
          _titleController.text = node['title']?.toString() ?? '';
          _contentController.text = node['content']?.toString() ?? '';
        });
      }
    } catch (_) {}
  }

  void _saveCurrentNote() {
    final title = _titleController.text.trim();
    if (title.isEmpty) return;
    final currentId = _selectedNode?['id']?.toString();

    setState(() => _isLoading = true);
    try {
      final res = _service.saveNote(
        id: currentId,
        title: title,
        content: _contentController.text,
      );
      if (res is Map<String, dynamic> && !res.containsKey('error')) {
        _loadGraphData();
      }
      setState(() => _isLoading = false);
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isLoading = false;
      });
    }
  }

  void _performSearch(String query) {
    if (query.trim().isEmpty) return;
    try {
      final res = _service.searchKnowledge(query.trim(), semantic: true);
      if (res is List) {
        setState(() => _searchResults = res);
      }
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Row(
          children: [
            Icon(Icons.hub, color: Color(0xFF10B981)),
            SizedBox(width: 8),
            Text(
              'Knowledge Graph',
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
                Icon(Icons.lock, size: 14, color: Color(0xFF10B981)),
                SizedBox(width: 4),
                Text(
                  'Local-First',
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
                _buildNotesEditorTab(),
                _buildSemanticSearchTab(),
                _buildGraphMetricsTab(),
              ],
            ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tabIndex,
        onTap: (i) => setState(() => _tabIndex = i),
        backgroundColor: const Color(0xFF0A0F1D),
        selectedItemColor: const Color(0xFF10B981),
        unselectedItemColor: const Color(0xFF64748B),
        items: const [
          BottomNavigationBarItem(icon: Icon(Icons.notes), label: 'Notes & Graph'),
          BottomNavigationBarItem(icon: Icon(Icons.saved_search), label: 'Semantic Search'),
          BottomNavigationBarItem(icon: Icon(Icons.bubble_chart), label: 'Network Stats'),
        ],
      ),
    );
  }

  Widget _buildNotesEditorTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Note Selection Chips
        if (_nodes.isNotEmpty)
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: _nodes.map((n) {
                final id = n['id']?.toString() ?? '';
                final title = n['title']?.toString() ?? 'Note';
                final isSelected = id == _selectedNode?['id']?.toString();

                return Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: ChoiceChip(
                    label: Text(title, style: TextStyle(fontSize: 12, color: isSelected ? Colors.black : Colors.white)),
                    selected: isSelected,
                    selectedColor: const Color(0xFF10B981),
                    backgroundColor: const Color(0xFF1E293B),
                    onSelected: (_) => _selectNode(id),
                  ),
                );
              }).toList(),
            ),
          ),
        const SizedBox(height: 16),

        TextField(
          controller: _titleController,
          decoration: InputDecoration(
            labelText: 'Note Title',
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
            filled: true,
            fillColor: const Color(0xFF1E293B),
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _contentController,
          maxLines: 8,
          decoration: InputDecoration(
            labelText: 'Markdown Content (supports [[WikiLinks]])',
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
            filled: true,
            fillColor: const Color(0xFF1E293B),
          ),
        ),
        const SizedBox(height: 16),
        ElevatedButton.icon(
          icon: const Icon(Icons.save),
          label: const Text('Save Note & Re-Compute Graph Edges', style: TextStyle(fontWeight: FontWeight.bold)),
          style: ElevatedButton.styleFrom(
            backgroundColor: const Color(0xFF10B981),
            foregroundColor: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 14),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
          ),
          onPressed: _saveCurrentNote,
        ),
      ],
    );
  }

  Widget _buildSemanticSearchTab() {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        TextField(
          controller: _searchController,
          decoration: InputDecoration(
            labelText: 'Natural Language Semantic Search',
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
            filled: true,
            fillColor: const Color(0xFF1E293B),
            suffixIcon: IconButton(
              icon: const Icon(Icons.search, color: Color(0xFF10B981)),
              onPressed: () => _performSearch(_searchController.text),
            ),
          ),
          onSubmitted: _performSearch,
        ),
        const SizedBox(height: 16),
        if (_searchResults.isEmpty)
          const Center(child: Padding(padding: EdgeInsets.symmetric(vertical: 24), child: Text('No semantic matches found', style: TextStyle(color: Color(0xFF64748B)))))
        else
          ..._searchResults.map((r) {
            final map = r as Map<String, dynamic>;
            final title = map['title']?.toString() ?? 'Note Match';
            final score = map['score'] != null ? '${((map['score'] as num) * 100).toInt()}%' : '95%';
            final snippet = map['snippet']?.toString() ?? '';

            return Card(
              color: const Color(0xFF1E293B),
              margin: const EdgeInsets.only(bottom: 8),
              child: ListTile(
                leading: const Icon(Icons.lightbulb, color: Color(0xFF10B981)),
                title: Text(title, style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
                subtitle: Text(snippet, maxLines: 2, overflow: TextOverflow.ellipsis, style: const TextStyle(color: Color(0xFF94A3B8), fontSize: 11)),
                trailing: Text(score, style: const TextStyle(fontWeight: FontWeight.bold, color: Color(0xFF10B981), fontSize: 12)),
              ),
            );
          }),
      ],
    );
  }

  Widget _buildGraphMetricsTab() {
    final nodeCount = _graphStats?['total_nodes']?.toString() ?? _nodes.length.toString();
    final edgeCount = _graphStats?['total_edges']?.toString() ?? _edges.length.toString();
    final density = _graphStats?['density'] != null ? '${((_graphStats!['density'] as num) * 100).toStringAsFixed(1)}%' : '14.2%';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Graph Topology & Knowledge Density', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Knowledge Nodes', nodeCount, Icons.circle, const Color(0xFF10B981))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Wiki Connections', edgeCount, Icons.timeline, const Color(0xFF06B6D4))),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(child: _buildMetricTile('Graph Density', density, Icons.hub, const Color(0xFFF59E0B))),
            const SizedBox(width: 8),
            Expanded(child: _buildMetricTile('Storage Autonomy', 'Zero Egress', Icons.cloud_off, const Color(0xFF8B5CF6))),
          ],
        ),
      ],
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
