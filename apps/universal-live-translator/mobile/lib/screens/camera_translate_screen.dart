import 'package:flutter/material.dart';

class CameraTranslateScreen extends StatefulWidget {
  const CameraTranslateScreen({super.key});

  @override
  State<CameraTranslateScreen> createState() => _CameraTranslateScreenState();
}

class _CameraTranslateScreenState extends State<CameraTranslateScreen> {
  bool _scanning = false;
  String _sampleResult = '';

  void _simulateScan() {
    setState(() => _scanning = true);
    Future.delayed(const Duration(milliseconds: 600), () {
      setState(() {
        _scanning = false;
        _sampleResult = 'EXIT / SALIDA\nEMERGENCY / EMERGENCIA';
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Camera OCR Overlay'),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Container(
              width: 300,
              height: 240,
              decoration: BoxDecoration(
                color: Colors.black26,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: Colors.white24, width: 2),
              ),
              child: _scanning
                  ? const Center(child: CircularProgressIndicator())
                  : Center(
                      child: Text(
                        _sampleResult.isEmpty
                            ? 'Point camera at street signs or documents'
                            : _sampleResult,
                        textAlign: TextAlign.center,
                        style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
                      ),
                    ),
            ),
            const SizedBox(height: 24),
            ElevatedButton.icon(
              icon: const Icon(Icons.camera_alt),
              label: const Text('Capture & Translate'),
              onPressed: _simulateScan,
            ),
          ],
        ),
      ),
    );
  }
}

