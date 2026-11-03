import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/translation_models.dart';
import '../services/ult_service.dart';

class PhrasebookScreen extends StatefulWidget {
  const PhrasebookScreen({super.key});

  @override
  State<PhrasebookScreen> createState() => _PhrasebookScreenState();
}

class _PhrasebookScreenState extends State<PhrasebookScreen> {
  List<PhrasebookItem> _phrases = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  void _load() async {
    final service = context.read<UltService>();
    final items = await service.listPhrases();
    setState(() {
      _phrases = items;
      _loading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Offline Phrasebook'),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _phrases.isEmpty
              ? const Center(child: Text('No saved phrases.'))
              : ListView.builder(
                  itemCount: _phrases.length,
                  itemBuilder: (context, index) {
                    final item = _phrases[index];
                    return ListTile(
                      title: Text(item.sourceText),
                      subtitle: Text(
                        item.targetText,
                        style: const TextStyle(color: Colors.cyanAccent),
                      ),
                      trailing: Icon(
                        item.favorite ? Icons.star : Icons.star_border,
                        color: Colors.amber,
                      ),
                    );
                  },
                ),
    );
  }
}

