import 'package:flutter/material.dart';
import '../models/explorer_models.dart';
import '../services/afe_service.dart';

class ContentSearchScreen extends StatefulWidget {
  final AfeService service;

  const ContentSearchScreen({super.key, required this.service});

  @override
  State<ContentSearchScreen> createState() => _ContentSearchScreenState();
}

class _ContentSearchScreenState extends State<ContentSearchScreen> {
  final TextEditingController _queryController = TextEditingController();
  String _selectedMode = 'Combined';
  ContentSearchResult? _searchResult;
  bool _isSearching = false;
  String? _errorMessage;

  Future<void> _performSearch() async {
    final query = _queryController.text.trim();
    if (query.isEmpty) return;

    setState(() {
      _isSearching = true;
      _errorMessage = null;
    });

    try {
      final res = await widget.service.searchContent(query, mode: _selectedMode);
      setState(() {
        _searchResult = res;
        _isSearching = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isSearching = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Semantic & Content Search'),
      ),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                TextField(
                  controller: _queryController,
                  decoration: InputDecoration(
                    hintText: 'Search keywords, concepts, synonyms...',
                    prefixIcon: const Icon(Icons.search),
                    suffixIcon: IconButton(
                      icon: const Icon(Icons.clear),
                      onPressed: () => _queryController.clear(),
                    ),
                    border: const OutlineInputBorder(),
                  ),
                  onSubmitted: (_) => _performSearch(),
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    const Text('Mode: ', style: TextStyle(fontWeight: FontWeight.bold)),
                    const SizedBox(width: 8),
                    SegmentedButton<String>(
                      segments: const [
                        ButtonSegment(value: 'Keyword', label: Text('Keyword')),
                        ButtonSegment(value: 'Semantic', label: Text('Semantic')),
                        ButtonSegment(value: 'Combined', label: Text('Hybrid')),
                      ],
                      selected: {_selectedMode},
                      onSelectionChanged: (set) {
                        setState(() => _selectedMode = set.first);
                      },
                    ),
                  ],
                ),
              ],
            ),
          ),
          if (_isSearching)
            const Expanded(child: Center(child: CircularProgressIndicator()))
          else if (_errorMessage != null)
            Expanded(
              child: Center(
                child: Text('Error: $_errorMessage',
                    style: const TextStyle(color: Colors.red)),
              ),
            )
          else if (_searchResult != null) ...[
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: Theme.of(context).colorScheme.surfaceVariant,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text('Matches: ${_searchResult!.totalMatches}'),
                  Text('${_searchResult!.executionTimeMs.toStringAsFixed(1)} ms'),
                ],
              ),
            ),
            Expanded(
              child: ListView.separated(
                itemCount: _searchResult!.items.length,
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemBuilder: (context, index) {
                  final item = _searchResult!.items[index];
                  return ListTile(
                    leading: CircleAvatar(
                      child: Text(item.format.toUpperCase(),
                          style: const TextStyle(fontSize: 10)),
                    ),
                    title: Text(item.fileName),
                    subtitle: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          item.snippet,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(fontSize: 12),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          item.path,
                          style: TextStyle(
                            fontSize: 10,
                            color: Colors.grey.shade600,
                            fontFamily: 'monospace',
                          ),
                        ),
                      ],
                    ),
                    trailing: Chip(
                      label: Text('${(item.score * 100).toInt()}%'),
                    ),
                  );
                },
              ),
            ),
          ] else
            const Expanded(
              child: Center(
                child: Text('Enter a query and search'),
              ),
            ),
        ],
      ),
    );
  }
}

