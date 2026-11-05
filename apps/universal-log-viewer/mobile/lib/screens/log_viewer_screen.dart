import 'package:flutter/material.dart';
import '../models/log_models.dart';
import '../services/ulv_service.dart';

class LogViewerScreen extends StatefulWidget {
  final UlvService service;
  final String? initialFilePath;

  const LogViewerScreen({
    Key? key,
    required this.service,
    this.initialFilePath,
  }) : super(key: key);

  @override
  State<LogViewerScreen> createState() => _LogViewerScreenState();
}

class _LogViewerScreenState extends State<LogViewerScreen> {
  OpenedFileInfo? _fileInfo;
  bool _isLoading = true;
  String? _errorMessage;

  final ScrollController _scrollController = ScrollController();
  final TextEditingController _searchController = TextEditingController();

  List<LogLineRecord> _visibleLines = [];
  int _firstVisibleLine = 0;
  static const int _batchSize = 100;

  bool _isRegex = false;
  bool _caseSensitive = false;
  LogSearchReport? _searchReport;
  bool _isSearching = false;

  TimelineHistogram? _timeline;
  List<LogTemplateCluster> _templates = [];
  List<ErrorCluster> _errorClusters = [];

  @override
  void initState() {
    super.initState();
    _loadFile(widget.initialFilePath ?? 'production_gateway.log');
    _scrollController.addListener(_onScroll);
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    _scrollController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _loadFile(String path) async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final info = await widget.service.open(path);
      final initialLines = await widget.service.getLinesSlice(0, _batchSize);
      final timeline = await widget.service.getTimelineHistogram(targetBuckets: 40);
      final templates = await widget.service.mineTemplates();
      final errors = await widget.service.clusterErrors();

      setState(() {
        _fileInfo = info;
        _visibleLines = initialLines;
        _firstVisibleLine = 0;
        _timeline = timeline;
        _templates = templates;
        _errorClusters = errors;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isLoading = false;
      });
    }
  }

  void _onScroll() {
    if (!_scrollController.hasClients || _fileInfo == null) return;
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;

    if (maxScroll > 0) {
      final fraction = (currentScroll / maxScroll).clamp(0.0, 1.0);
      final estimatedLine = (fraction * (_fileInfo!.totalLines - _batchSize)).toInt();
      final newLine = estimatedLine.clamp(0, (_fileInfo!.totalLines - _batchSize).clamp(0, _fileInfo!.totalLines));

      if ((newLine - _firstVisibleLine).abs() > 20) {
        _fetchSlice(newLine);
      }
    }
  }

  Future<void> _fetchSlice(int startLine) async {
    try {
      final slice = await widget.service.getLinesSlice(startLine, _batchSize);
      setState(() {
        _firstVisibleLine = startLine;
        _visibleLines = slice;
      });
    } catch (_) {}
  }

  Future<void> _performSearch() async {
    final query = _searchController.text.trim();
    if (query.isEmpty) {
      setState(() {
        _searchReport = null;
      });
      return;
    }

    setState(() {
      _isSearching = true;
    });

    try {
      final report = await widget.service.search(
        query,
        isRegex: _isRegex,
        caseSensitive: _caseSensitive,
      );

      setState(() {
        _searchReport = report;
        _isSearching = false;
      });

      if (report.matches.isNotEmpty) {
        _jumpToLine(report.matches.first.lineNumber);
      }
    } catch (e) {
      setState(() {
        _isSearching = false;
      });
    }
  }

  void _jumpToLine(int lineNumber) {
    if (_fileInfo == null || _fileInfo!.totalLines == 0) return;
    _fetchSlice(lineNumber);
    if (_scrollController.hasClients) {
      final fraction = (lineNumber / _fileInfo!.totalLines).clamp(0.0, 1.0);
      final targetOffset = fraction * _scrollController.position.maxScrollExtent;
      _scrollController.jumpTo(targetOffset);
    }
  }

  void _showOpenFileModal() {
    final controller = TextEditingController(text: _fileInfo?.filePath ?? '');
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: const Color(0xFF161B22),
        title: const Text('Open Log File / Stream', style: TextStyle(color: Colors.white)),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text(
              'Enter an absolute file path or directory to load logs offline.',
              style: TextStyle(color: Colors.white70, fontSize: 12),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: controller,
              style: const TextStyle(color: Colors.white, fontSize: 13),
              decoration: const InputDecoration(
                hintText: '/path/to/logfile.log',
                filled: true,
                fillColor: Color(0xFF0F141C),
                border: OutlineInputBorder(),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              final path = controller.text.trim();
              if (path.isNotEmpty) {
                Navigator.pop(ctx);
                _loadFile(path);
              }
            },
            child: const Text('Open'),
          ),
        ],
      ),
    );
  }

  void _showTraceWaterfallModal() {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF161B22),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Row(
              children: [
                Icon(Icons.waterfall_chart, color: Color(0xFF38BDF8)),
                SizedBox(width: 8),
                Text(
                  'OpenTelemetry Trace Waterfall',
                  style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16),
                ),
              ],
            ),
            const SizedBox(height: 12),
            const Text(
              'Correlates spans and HTTP/gRPC traces chronologically to detect latency bottlenecks.',
              style: TextStyle(color: Colors.white60, fontSize: 12),
            ),
            const SizedBox(height: 16),
            FutureBuilder<Map<String, dynamic>>(
              future: widget.service.buildWaterfall(),
              builder: (ctx, snapshot) {
                if (snapshot.connectionState == ConnectionState.waiting) {
                  return const Center(child: CircularProgressIndicator());
                }
                final traces = (snapshot.data?['traces'] as List<dynamic>?) ?? [];
                if (traces.isEmpty) {
                  return const Padding(
                    padding: EdgeInsets.symmetric(vertical: 20),
                    child: Center(
                      child: Text(
                        'No trace spans detected in current log stream',
                        style: TextStyle(color: Colors.white54),
                      ),
                    ),
                  );
                }
                return SizedBox(
                  height: 200,
                  child: ListView.builder(
                    itemCount: traces.length,
                    itemBuilder: (ctx, i) {
                      final span = traces[i];
                      return ListTile(
                        leading: const Icon(Icons.hub_outlined, color: Colors.cyanAccent),
                        title: Text(span.toString(), style: const TextStyle(color: Colors.white, fontSize: 12)),
                      );
                    },
                  ),
                );
              },
            ),
          ],
        ),
      ),
    );
  }

  void _showFieldAggregatorModal() {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF161B22),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Row(
              children: [
                Icon(Icons.pie_chart_outline, color: Color(0xFF38BDF8)),
                SizedBox(width: 8),
                Text(
                  'Structured Field Frequency Aggregator',
                  style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16),
                ),
              ],
            ),
            const SizedBox(height: 12),
            FutureBuilder<Map<String, dynamic>>(
              future: widget.service.aggregateFields('level'),
              builder: (ctx, snapshot) {
                if (snapshot.connectionState == ConnectionState.waiting) {
                  return const Center(child: CircularProgressIndicator());
                }
                final freqs = (snapshot.data?['frequencies'] as Map<String, dynamic>?) ?? {};
                return Column(
                  children: freqs.entries.map((e) {
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 4),
                      child: Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                            decoration: BoxDecoration(
                              color: Colors.blueGrey.withOpacity(0.3),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(e.key, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
                          ),
                          const Spacer(),
                          Text('${e.value} occurrences', style: const TextStyle(color: Colors.white70)),
                        ],
                      ),
                    );
                  }).toList(),
                );
              },
            ),
          ],
        ),
      ),
    );
  }

  void _openAiInsightsSheet() {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1E232A),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => DefaultTabController(
        length: 2,
        child: Padding(
          padding: const EdgeInsets.all(16.0),
          child: Column(
            children: [
              Container(
                width: 40,
                height: 4,
                margin: const EdgeInsets.only(bottom: 12),
                decoration: BoxDecoration(
                  color: Colors.white24,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const TabBar(
                indicatorColor: Color(0xFF38BDF8),
                tabs: [
                  Tab(text: 'Drain Templates', icon: Icon(Icons.hub_outlined, size: 20)),
                  Tab(text: 'Error Clusters', icon: Icon(Icons.error_outline, size: 20)),
                ],
              ),
              Expanded(
                child: TabBarView(
                  children: [
                    // Drain Templates Tab
                    ListView.builder(
                      itemCount: _templates.length,
                      itemBuilder: (ctx, i) {
                        final t = _templates[i];
                        return Card(
                          color: const Color(0xFF12161C),
                          margin: const EdgeInsets.symmetric(vertical: 6),
                          child: ListTile(
                            title: Text(
                              t.template,
                              style: const TextStyle(
                                fontFamily: 'monospace',
                                fontSize: 13,
                                color: Colors.white,
                              ),
                            ),
                            subtitle: Text(
                              'Frequency: ${t.frequency} matches',
                              style: const TextStyle(color: Colors.white60, fontSize: 12),
                            ),
                            trailing: TextButton(
                              onPressed: () {
                                Navigator.pop(ctx);
                                if (t.sampleLines.isNotEmpty) {
                                  _jumpToLine(t.sampleLines.first);
                                }
                              },
                              child: const Text('Jump'),
                            ),
                          ),
                        );
                      },
                    ),

                    // Error Clusters Tab
                    ListView.builder(
                      itemCount: _errorClusters.length,
                      itemBuilder: (ctx, i) {
                        final ec = _errorClusters[i];
                        return Card(
                          color: const Color(0xFF12161C),
                          margin: const EdgeInsets.symmetric(vertical: 6),
                          child: ListTile(
                            leading: const CircleAvatar(
                              backgroundColor: Colors.redAccent,
                              radius: 12,
                              child: Icon(Icons.close, size: 14, color: Colors.white),
                            ),
                            title: Text(
                              ec.template,
                              style: const TextStyle(
                                fontFamily: 'monospace',
                                fontSize: 13,
                                color: Colors.white,
                              ),
                            ),
                            subtitle: Text(
                              'Count: ${ec.errorCount} | Sources: ${ec.affectedSources.join(', ')}',
                              style: const TextStyle(color: Colors.white60, fontSize: 12),
                            ),
                            trailing: TextButton(
                              onPressed: () {
                                Navigator.pop(ctx);
                                if (ec.sampleLines.isNotEmpty) {
                                  _jumpToLine(ec.sampleLines.first);
                                }
                              },
                              child: const Text('Jump'),
                            ),
                          ),
                        );
                      },
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Color _getSeverityColor(String text) {
    if (text.contains('[ERROR]') || text.contains('[FATAL]')) {
      return Colors.redAccent;
    } else if (text.contains('[WARN]')) {
      return Colors.amberAccent;
    } else if (text.contains('[INFO]')) {
      return const Color(0xFF38BDF8);
    }
    return Colors.white54;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0F141C),
      appBar: AppBar(
        backgroundColor: const Color(0xFF161B22),
        elevation: 0,
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Universal Log Viewer',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
            if (_fileInfo != null)
              Text(
                '${_fileInfo!.totalLines} lines • ${(_fileInfo!.fileSizeBytes / 1024).toStringAsFixed(1)} KB',
                style: const TextStyle(fontSize: 11, color: Colors.white60),
              ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.folder_open, color: Color(0xFF38BDF8)),
            onPressed: _showOpenFileModal,
            tooltip: 'Open Log File / Stream',
          ),
          IconButton(
            icon: const Icon(Icons.waterfall_chart, color: Color(0xFF38BDF8)),
            onPressed: _showTraceWaterfallModal,
            tooltip: 'OpenTelemetry Trace Waterfall',
          ),
          IconButton(
            icon: const Icon(Icons.pie_chart_outline, color: Color(0xFF38BDF8)),
            onPressed: _showFieldAggregatorModal,
            tooltip: 'Field Frequency Aggregator',
          ),
          IconButton(
            icon: const Icon(Icons.insights_outlined, color: Color(0xFF38BDF8)),
            onPressed: _openAiInsightsSheet,
            tooltip: 'AI Root-Cause & Drain Insights',
          ),
        ],
      ),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : _errorMessage != null
              ? Center(
                  child: Text(
                    'Error: $_errorMessage',
                    style: const TextStyle(color: Colors.redAccent),
                  ),
                )
              : Column(
                  children: [
                    // Search bar
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                      color: const Color(0xFF161B22),
                      child: Row(
                        children: [
                          Expanded(
                            child: TextField(
                              controller: _searchController,
                              style: const TextStyle(color: Colors.white, fontSize: 14),
                              decoration: InputDecoration(
                                hintText: 'Search logs (sub-50ms)...',
                                hintStyle: const TextStyle(color: Colors.white38),
                                prefixIcon: const Icon(Icons.search, size: 20, color: Colors.white54),
                                suffixIcon: _isSearching
                                    ? const Padding(
                                        padding: EdgeInsets.all(12),
                                        child: SizedBox(
                                          width: 16,
                                          height: 16,
                                          child: CircularProgressIndicator(strokeWidth: 2),
                                        ),
                                      )
                                    : IconButton(
                                        icon: const Icon(Icons.arrow_forward, size: 20, color: Colors.white54),
                                        onPressed: _performSearch,
                                      ),
                                filled: true,
                                fillColor: const Color(0xFF0F141C),
                                border: OutlineInputBorder(
                                  borderRadius: BorderRadius.circular(8),
                                  borderSide: BorderSide.none,
                                ),
                                isDense: true,
                                contentPadding: const EdgeInsets.symmetric(vertical: 8),
                              ),
                              onSubmitted: (_) => _performSearch(),
                            ),
                          ),
                          const SizedBox(width: 6),
                          FilterChip(
                            label: const Text('.*', style: TextStyle(fontSize: 12)),
                            selected: _isRegex,
                            onSelected: (v) => setState(() => _isRegex = v),
                            visualDensity: VisualDensity.compact,
                          ),
                          const SizedBox(width: 4),
                          FilterChip(
                            label: const Text('Aa', style: TextStyle(fontSize: 12)),
                            selected: _caseSensitive,
                            onSelected: (v) => setState(() => _caseSensitive = v),
                            visualDensity: VisualDensity.compact,
                          ),
                        ],
                      ),
                    ),

                    // Timeline histogram overview
                    if (_timeline != null && _timeline!.buckets.isNotEmpty)
                      Container(
                        height: 36,
                        color: const Color(0xFF1A202C),
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.end,
                          children: _timeline!.buckets.map((b) {
                            final maxTotal = _timeline!.buckets
                                .map((e) => e.totalCount)
                                .reduce((a, b) => a > b ? a : b);
                            final heightFrac = maxTotal > 0 ? (b.totalCount / maxTotal) : 0.1;
                            final hasErr = b.errorCount > 0;
                            return Expanded(
                              child: GestureDetector(
                                onTap: () {
                                  // Jump to approximate timeline position
                                  final frac = b.bucketIndex / _timeline!.buckets.length;
                                  final targetLine = ((_fileInfo?.totalLines ?? 0) * frac).toInt();
                                  _jumpToLine(targetLine);
                                },
                                child: Container(
                                  margin: const EdgeInsets.symmetric(horizontal: 0.5),
                                  height: (28 * heightFrac).clamp(3.0, 28.0),
                                  decoration: BoxDecoration(
                                    color: hasErr ? Colors.redAccent : const Color(0xFF38BDF8),
                                    borderRadius: BorderRadius.circular(1),
                                  ),
                                ),
                              ),
                            );
                          }).toList(),
                        ),
                      ),

                    // Virtualized log row list
                    Expanded(
                      child: ListView.builder(
                        controller: _scrollController,
                        itemCount: _fileInfo?.totalLines ?? 0,
                        itemExtent: 32,
                        itemBuilder: (context, index) {
                          final localIndex = index - _firstVisibleLine;
                          final record = (localIndex >= 0 && localIndex < _visibleLines.length)
                              ? _visibleLines[localIndex]
                              : null;

                          final text = record?.text ?? 'Line $index (buffering...)';
                          final sevColor = _getSeverityColor(text);

                          return Container(
                            height: 32,
                            padding: const EdgeInsets.symmetric(horizontal: 8),
                            decoration: BoxDecoration(
                              border: Border(
                                bottom: BorderSide(color: Colors.white.withOpacity(0.04)),
                              ),
                            ),
                            child: Row(
                              children: [
                                SizedBox(
                                  width: 48,
                                  child: Text(
                                    '${index + 1}',
                                    style: const TextStyle(
                                      color: Colors.white30,
                                      fontFamily: 'monospace',
                                      fontSize: 11,
                                    ),
                                  ),
                                ),
                                Container(
                                  width: 4,
                                  height: 14,
                                  margin: const EdgeInsets.only(right: 8),
                                  decoration: BoxDecoration(
                                    color: sevColor,
                                    borderRadius: BorderRadius.circular(2),
                                  ),
                                ),
                                Expanded(
                                  child: Text(
                                    text,
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                    style: TextStyle(
                                      color: record != null ? Colors.white70 : Colors.white24,
                                      fontFamily: 'monospace',
                                      fontSize: 12,
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          );
                        },
                      ),
                    ),
                  ],
                ),
    );
  }
}

