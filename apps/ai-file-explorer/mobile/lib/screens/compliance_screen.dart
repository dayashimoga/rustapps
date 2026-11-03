import 'package:flutter/material.dart';
import '../models/explorer_models.dart';
import '../services/afe_service.dart';

class ComplianceScreen extends StatefulWidget {
  final AfeService service;

  const ComplianceScreen({super.key, required this.service});

  @override
  State<ComplianceScreen> createState() => _ComplianceScreenState();
}

class _ComplianceScreenState extends State<ComplianceScreen> {
  final TextEditingController _pathController = TextEditingController(
    text: '/storage/emulated/0/Documents',
  );
  ComplianceScanSummary? _summary;
  bool _isScanning = false;
  String? _errorMessage;
  String _selectedSeverity = 'all';

  Future<void> _startScan() async {
    final path = _pathController.text.trim();
    if (path.isEmpty) return;

    setState(() {
      _isScanning = true;
      _errorMessage = null;
    });

    try {
      final res = await widget.service.scanCompliance(path);
      setState(() {
        _summary = res;
        _isScanning = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isScanning = false;
      });
    }
  }

  Color _severityColor(String sev) {
    switch (sev.toLowerCase()) {
      case 'critical':
        return Colors.red;
      case 'high':
        return Colors.orange;
      case 'medium':
        return Colors.blue;
      default:
        return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final filtered = _summary?.findings.filter((f) {
          if (_selectedSeverity == 'all') return true;
          return f.severity.toLowerCase() == _selectedSeverity;
        }).toList() ??
        [];

    return Scaffold(
      appBar: AppBar(
        title: const Text('PII Compliance Audit'),
        actions: const [
          Padding(
            padding: EdgeInsets.symmetric(horizontal: 12),
            child: Chip(
              label: Text('Business Tier', style: TextStyle(fontSize: 10)),
              backgroundColor: Colors.teal,
            ),
          ),
        ],
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  controller: _pathController,
                  decoration: const InputDecoration(
                    labelText: 'Directory or File Path to Audit',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.security),
                  ),
                ),
                const SizedBox(height: 12),
                SizedBox(
                  width: double.infinity,
                  child: ElevatedButton.icon(
                    icon: const Icon(Icons.shield),
                    label: const Text('Run Offline Compliance Audit'),
                    onPressed: _isScanning ? null : _startScan,
                  ),
                ),
              ],
            ),
          ),
          if (_isScanning)
            const Expanded(child: Center(child: CircularProgressIndicator()))
          else if (_errorMessage != null)
            Expanded(
              child: Center(
                child: Text('Error: $_errorMessage',
                    style: const TextStyle(color: Colors.red)),
              ),
            )
          else if (_summary != null) ...[
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Row(
                children: [
                  _statBadge('Scanned', '${_summary!.scannedFilesCount}'),
                  const SizedBox(width: 8),
                  _statBadge('Violations', '${_summary!.findingsCount}',
                      color: _summary!.findingsCount > 0
                          ? Colors.red.shade100
                          : Colors.green.shade100),
                  const SizedBox(width: 8),
                  _statBadge('Critical', '${_summary!.criticalCount}',
                      color: Colors.red.shade100),
                  const SizedBox(width: 8),
                  _statBadge('Latency',
                      '${_summary!.scanDurationMs.toStringAsFixed(1)}ms'),
                ],
              ),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: filtered.isEmpty
                  ? const Center(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(Icons.check_circle, color: Colors.green, size: 48),
                          SizedBox(height: 12),
                          Text('No Violations Found'),
                        ],
                      ),
                    )
                  : ListView.separated(
                      itemCount: filtered.length,
                      separatorBuilder: (_, __) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final finding = filtered[index];
                        return ListTile(
                          leading: Chip(
                            label: Text(
                              finding.severity.toUpperCase(),
                              style: const TextStyle(
                                  color: Colors.white, fontSize: 10),
                            ),
                            backgroundColor: _severityColor(finding.severity),
                          ),
                          title: Text(finding.dataType.replaceAll('_', ' ').toUpperCase()),
                          subtitle: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                '${finding.filePath} (line ${finding.lineNumber})',
                                style: const TextStyle(
                                  fontSize: 11,
                                  fontFamily: 'monospace',
                                ),
                              ),
                              const SizedBox(height: 4),
                              Container(
                                padding: const EdgeInsets.all(6),
                                decoration: BoxDecoration(
                                  color: Colors.grey.shade900,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  finding.redactedPreview,
                                  style: const TextStyle(
                                    fontFamily: 'monospace',
                                    fontSize: 11,
                                    color: Colors.greenAccent,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        );
                      },
                    ),
            ),
          ] else
            const Expanded(
              child: Center(
                child: Text('Enter a directory and run compliance scan'),
              ),
            ),
        ],
      ),
    );
  }

  Widget _statBadge(String label, String value, {Color? color}) {
    return Expanded(
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 8),
        decoration: BoxDecoration(
          color: color ?? Colors.grey.shade200,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Column(
          children: [
            Text(value,
                style: const TextStyle(
                    fontWeight: FontWeight.bold, fontSize: 14)),
            Text(label, style: const TextStyle(fontSize: 10)),
          ],
        ),
      ),
    );
  }
}

extension _ListFilter<T> on List<T> {
  Iterable<T> filter(bool Function(T) predicate) => where(predicate);
}

