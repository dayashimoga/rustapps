import 'package:flutter/material.dart';
import '../models/explorer_models.dart';
import '../services/afe_service.dart';

class BrowserScreen extends StatefulWidget {
  final AfeService service;

  const BrowserScreen({super.key, required this.service});

  @override
  State<BrowserScreen> createState() => _BrowserScreenState();
}

class _BrowserScreenState extends State<BrowserScreen> {
  String _currentPath = '/storage/emulated/0';
  DirectoryListing? _listing;
  bool _isLoading = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _loadDirectory(_currentPath);
  }

  Future<void> _loadDirectory(String path) async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final listing = await widget.service.listDirectory(path);
      setState(() {
        _currentPath = path;
        _listing = listing;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isLoading = false;
      });
    }
  }

  void _navigateUp() {
    if (_listing?.parentPath != null) {
      _loadDirectory(_listing!.parentPath!);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_currentPath.split('/').last.isEmpty
            ? 'Root'
            : _currentPath.split('/').last),
        leading: _listing?.parentPath != null
            ? IconButton(
                icon: const Icon(Icons.arrow_upward),
                onPressed: _navigateUp,
                tooltip: 'Navigate Up',
              )
            : null,
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => _loadDirectory(_currentPath),
            tooltip: 'Refresh',
          ),
        ],
      ),
      body: Column(
        children: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            color: Theme.of(context).colorScheme.surfaceVariant,
            width: double.infinity,
            child: Text(
              _currentPath,
              style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          if (_isLoading)
            const Expanded(child: Center(child: CircularProgressIndicator()))
          else if (_errorMessage != null)
            Expanded(
              child: Center(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Text(
                    'Error: $_errorMessage',
                    style: const TextStyle(color: Colors.red),
                  ),
                ),
              ),
            )
          else if (_listing == null || _listing!.items.isEmpty)
            const Expanded(
              child: Center(child: Text('Folder is empty')),
            )
          else
            Expanded(
              child: ListView.separated(
                itemCount: _listing!.items.length,
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemBuilder: (context, index) {
                  final item = _listing!.items[index];
                  return ListTile(
                    leading: Icon(
                      item.isDir ? Icons.folder : Icons.insert_drive_file,
                      color: item.isDir ? Colors.amber : Colors.blueGrey,
                    ),
                    title: Text(item.fileName),
                    subtitle: Text(
                      item.isDir ? 'Directory' : item.humanReadableSize,
                      style: const TextStyle(fontSize: 12),
                    ),
                    trailing: item.isDir
                        ? const Icon(Icons.chevron_right, size: 20)
                        : null,
                    onTap: () {
                      if (item.isDir) {
                        _loadDirectory(item.path);
                      }
                    },
                  );
                },
              ),
            ),
        ],
      ),
    );
  }
}

