import 'package:flutter/material.dart';
import '../models/explorer_models.dart';
import '../services/afe_service.dart';

class OcrScreen extends StatefulWidget {
  final AfeService service;

  const OcrScreen({super.key, required this.service});

  @override
  State<OcrScreen> createState() => _OcrScreenState();
}

class _OcrScreenState extends State<OcrScreen> {
  final TextEditingController _pathController = TextEditingController(
    text: '/storage/emulated/0/DCIM/receipt_scan.png',
  );
  OcrResult? _ocrResult;
  bool _isProcessing = false;
  String? _errorMessage;

  Future<void> _processOcr() async {
    final path = _pathController.text.trim();
    if (path.isEmpty) return;

    setState(() {
      _isProcessing = true;
      _errorMessage = null;
    });

    try {
      final res = await widget.service.ocrImage(path);
      setState(() {
        _ocrResult = res;
        _isProcessing = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = e.toString();
        _isProcessing = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Offline Image OCR'),
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            TextField(
              controller: _pathController,
              decoration: const InputDecoration(
                labelText: 'Image File Path (PNG/BMP)',
                border: OutlineInputBorder(),
                prefixIcon: Icon(Icons.image),
              ),
            ),
            const SizedBox(height: 12),
            SizedBox(
              width: double.infinity,
              child: ElevatedButton.icon(
                icon: const Icon(Icons.document_scanner),
                label: const Text('Extract Text via OCR'),
                onPressed: _isProcessing ? null : _processOcr,
              ),
            ),
            const SizedBox(height: 16),
            if (_isProcessing)
              const Expanded(child: Center(child: CircularProgressIndicator()))
            else if (_errorMessage != null)
              Expanded(
                child: Center(
                  child: Text('Error: $_errorMessage',
                      style: const TextStyle(color: Colors.red)),
                ),
              )
            else if (_ocrResult != null)
              Expanded(
                child: ListView(
                  children: [
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.spaceAround,
                          children: [
                            Text(
                              'Confidence: ${(_ocrResult!.confidence * 100).toStringAsFixed(1)}%',
                              style: const TextStyle(fontWeight: FontWeight.bold),
                            ),
                            Text('Lines: ${_ocrResult!.lineCount}'),
                            Text('${_ocrResult!.processingTimeMs.toStringAsFixed(1)} ms'),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),
                    const Text('Recognized Text:',
                        style: TextStyle(fontWeight: FontWeight.bold)),
                    const SizedBox(height: 6),
                    Container(
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: Colors.grey.shade900,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: SelectableText(
                        _ocrResult!.text,
                        style: const TextStyle(
                          fontFamily: 'monospace',
                          color: Colors.white,
                        ),
                      ),
                    ),
                  ],
                ),
              )
            else
              const Expanded(
                child: Center(child: Text('Select an image and run OCR')),
              ),
          ],
        ),
      ),
    );
  }
}

