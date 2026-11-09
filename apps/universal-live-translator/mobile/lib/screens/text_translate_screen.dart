import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/translation_models.dart';
import '../services/ult_service.dart';

class TextTranslateScreen extends StatefulWidget {
  const TextTranslateScreen({super.key});

  @override
  State<TextTranslateScreen> createState() => _TextTranslateScreenState();
}

class _TextTranslateScreenState extends State<TextTranslateScreen> {
  final TextEditingController _inputController = TextEditingController();
  String _sourceLang = 'auto';
  String _targetLang = 'es';
  String _outputText = '';
  LidResult? _detectedLid;
  bool _isTranslating = false;
  double _latencyMs = 0.0;

  void _translate() async {
    final text = _inputController.text.trim();
    if (text.isEmpty) {
      setState(() {
        _outputText = '';
        _detectedLid = null;
      });
      return;
    }

    setState(() => _isTranslating = true);
    final service = context.read<UltService>();

    if (_sourceLang == 'auto') {
      _detectedLid = await service.detectLanguage(text);
    }

    final req = TranslationRequest(
      text: text,
      sourceLang: _sourceLang == 'auto' ? null : _sourceLang,
      targetLang: _targetLang,
    );

    final res = await service.translateText(req);
    setState(() {
      _outputText = res.translatedText;
      _latencyMs = res.latencyMs;
      _isTranslating = false;
    });
  }

  void _swapLanguages() {
    if (_sourceLang != 'auto') {
      final temp = _sourceLang;
      setState(() {
        _sourceLang = _targetLang;
        _targetLang = temp;
        final tempText = _inputController.text;
        _inputController.text = _outputText;
        _outputText = tempText;
      });
      _translate();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Offline Text Translator'),
        actions: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            margin: const EdgeInsets.only(right: 12),
            decoration: BoxDecoration(
              color: Colors.green.withOpacity(0.2),
              borderRadius: BorderRadius.circular(4),
              border: Border.all(color: Colors.green),
            ),
            child: const Center(
              child: Text(
                '100% OFFLINE',
                style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold, color: Colors.greenAccent),
              ),
            ),
          )
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          children: [
            // Language selector toolbar
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                DropdownButton<String>(
                  value: _sourceLang,
                  dropdownColor: const Color(0xFF1E293B),
                  items: [
                    const DropdownMenuItem(value: 'auto', child: Text('Auto Detect')),
                    ...supportedLanguagesList.map(
                      (l) => DropdownMenuItem(value: l.code, child: Text(l.displayName)),
                    ),
                  ],
                  onChanged: (val) {
                    if (val != null) {
                      setState(() => _sourceLang = val);
                      _translate();
                    }
                  },
                ),
                IconButton(
                  icon: const Icon(Icons.swap_horiz),
                  onPressed: _swapLanguages,
                ),
                DropdownButton<String>(
                  value: _targetLang,
                  dropdownColor: const Color(0xFF1E293B),
                  items: supportedLanguagesList
                      .map((l) => DropdownMenuItem(value: l.code, child: Text(l.displayName)))
                      .toList(),
                  onChanged: (val) {
                    if (val != null) {
                      setState(() => _targetLang = val);
                      _translate();
                    }
                  },
                ),
              ],
            ),
            const SizedBox(height: 12),

            // Source Input Box
            Expanded(
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: const Color(0xFF0F172A),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: Colors.white12),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (_detectedLid != null && _sourceLang == 'auto')
                      Text(
                        'Detected: ${_detectedLid!.language.toUpperCase()} (${(_detectedLid!.confidence * 100).toInt()}%)',
                        style: const TextStyle(fontSize: 12, color: Colors.cyanAccent),
                      ),
                    Expanded(
                      child: TextField(
                        controller: _inputController,
                        maxLines: null,
                        expands: true,
                        decoration: const InputDecoration(
                          hintText: 'Enter text to translate offline...',
                          border: InputBorder.none,
                        ),
                        onChanged: (_) => _translate(),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 12),

            // Target Output Box
            Expanded(
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: const Color(0xFF1E293B),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: Colors.white12),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text('Translation', style: TextStyle(fontWeight: FontWeight.bold)),
                        if (_latencyMs > 0)
                          Text(
                            '${_latencyMs.toStringAsFixed(1)} ms',
                            style: const TextStyle(fontSize: 11, color: Colors.indigoAccent),
                          ),
                      ],
                    ),
                    const Divider(color: Colors.white10),
                    Expanded(
                      child: SingleChildScrollView(
                        child: Text(
                          _outputText.isEmpty
                              ? (_isTranslating ? 'Translating...' : 'Translation will appear here')
                              : _outputText,
                          style: TextStyle(
                            fontSize: 16,
                            color: _outputText.isEmpty ? Colors.white38 : Colors.white,
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

