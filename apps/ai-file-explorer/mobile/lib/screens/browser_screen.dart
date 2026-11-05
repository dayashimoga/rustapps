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
                      } else {
                        _showFileDetailsSheet(item);
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

  void _showFileDetailsSheet(FileItem item) {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1E293B),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) {
        final isArchive = item.extension != null &&
            ['zip', 'tar', 'gz', 'tgz'].contains(item.extension!.toLowerCase());
        return Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(
                    isArchive ? Icons.archive : Icons.insert_drive_file,
                    color: const Color(0xFF38BDF8),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      item.fileName,
                      style: const TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        fontSize: 16,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Text(
                'Path: ${item.path}',
                style: TextStyle(color: Colors.white.withOpacity(0.6), fontSize: 12),
              ),
              Text(
                'Size: ${item.humanReadableSize}',
                style: TextStyle(color: Colors.white.withOpacity(0.6), fontSize: 12),
              ),
              const SizedBox(height: 16),
              Row(
                children: [
                  Expanded(
                    child: OutlinedButton.icon(
                      icon: const Icon(Icons.fingerprint, size: 18),
                      label: const Text('BLAKE3 Hash'),
                      onPressed: () async {
                        Navigator.pop(ctx);
                        try {
                          final hash = await widget.service.computeFileHash(item.path);
                          if (mounted) {
                            ScaffoldMessenger.of(context).showSnackBar(
                              SnackBar(
                                content: Text('BLAKE3: $hash'),
                                duration: const Duration(seconds: 4),
                              ),
                            );
                          }
                        } catch (e) {
                          if (mounted) {
                            ScaffoldMessenger.of(context).showSnackBar(
                              SnackBar(content: Text('Hash error: $e')),
                            );
                          }
                        }
                      },
                    ),
                  ),
                  const SizedBox(width: 8),
                  if (isArchive)
                    Expanded(
                      child: ElevatedButton.icon(
                        icon: const Icon(Icons.folder_zip, size: 18),
                        label: const Text('Inspect'),
                        onPressed: () async {
                          Navigator.pop(ctx);
                          final entries = await widget.service.inspectArchive(item.path);
                          if (mounted) {
                            ScaffoldMessenger.of(context).showSnackBar(
                              SnackBar(
                                content: Text('Archive contains ${entries.length} entries'),
                              ),
                            );
                          }
                        },
                      ),
                    )
                  else
                    Expanded(
                      child: ElevatedButton.icon(
                        icon: const Icon(Icons.visibility, size: 18),
                        label: const Text('Preview'),
                        onPressed: () async {
                          Navigator.pop(ctx);
                          try {
                            final doc = await widget.service.extractDocument(item.path);
                            if (mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(
                                  content: Text('Extracted ${doc.metadata.wordCount} words (${doc.extension ?? "plain"})'),
                                ),
                              );
                            }
                          } catch (e) {
                            if (mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(content: Text('Extract error: $e')),
                              );
                            }
                          }
                        },
                      ),
                    ),
                ],
              ),
            ],
          ),
        );
      },
    );
  }
}

