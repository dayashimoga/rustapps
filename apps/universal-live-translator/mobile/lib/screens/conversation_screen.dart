import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/translation_models.dart';
import '../services/ult_service.dart';

class ConversationScreen extends StatefulWidget {
  const ConversationScreen({super.key});

  @override
  State<ConversationScreen> createState() => _ConversationScreenState();
}

class _ConversationScreenState extends State<ConversationScreen> {
  final TextEditingController _partyAController = TextEditingController();
  final TextEditingController _partyBController = TextEditingController();
  ConversationSession? _session;
  final List<ConversationTurn> _turns = [];

  @override
  void initState() {
    super.initState();
    _startSession();
  }

  void _startSession() async {
    final service = context.read<UltService>();
    final session = await service.startConversationSession('en', 'es');
    setState(() {
      _session = session;
    });
  }

  void _submitTurn(String speaker, String text) async {
    if (_session == null || text.trim().isEmpty) return;
    final service = context.read<UltService>();
    final turn = await service.addConversationTurn(_session!.id, speaker, text.trim());
    setState(() {
      _turns.add(turn);
      if (speaker == 'party_a') {
        _partyAController.clear();
      } else {
        _partyBController.clear();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Split-Screen Interpreter'),
      ),
      body: Column(
        children: [
          // Turns display
          Expanded(
            child: ListView.builder(
              padding: const EdgeInsets.all(12),
              itemCount: _turns.length,
              itemBuilder: (context, index) {
                final turn = _turns[index];
                final isA = turn.speaker == 'party_a';
                return Align(
                  alignment: isA ? Alignment.centerLeft : Alignment.centerRight,
                  child: Container(
                    margin: const EdgeInsets.symmetric(vertical: 4),
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: isA
                          ? Colors.indigo.withValues(alpha: 0.2)
                          : Colors.cyan.withValues(alpha: 0.2),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(
                        color: isA ? Colors.indigoAccent : Colors.cyanAccent,
                      ),
                    ),
                    child: Column(
                      crossAxisAlignment:
                          isA ? CrossAxisAlignment.start : CrossAxisAlignment.end,
                      children: [
                        Text(
                          isA ? 'Party A (EN)' : 'Party B (ES)',
                          style: const TextStyle(fontSize: 10, color: Colors.white60),
                        ),
                        Text(turn.originalText, style: const TextStyle(color: Colors.white70)),
                        const SizedBox(height: 4),
                        Text(
                          turn.translatedText,
                          style: const TextStyle(fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),

          // Split Input Controls
          Container(
            padding: const EdgeInsets.all(8),
            decoration: const BoxDecoration(
              color: Color(0xFF0F172A),
              border: Border(top: BorderSide(color: Colors.white12)),
            ),
            child: Column(
              children: [
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _partyAController,
                        decoration: const InputDecoration(
                          hintText: 'Party A (English)...',
                          isDense: true,
                        ),
                        onSubmitted: (val) => _submitTurn('party_a', val),
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.send, color: Colors.indigoAccent),
                      onPressed: () => _submitTurn('party_a', _partyAController.text),
                    ),
                  ],
                ),
                const Divider(color: Colors.white10),
                Row(
                  children: [
                    Expanded(
                      child: TextField(
                        controller: _partyBController,
                        decoration: const InputDecoration(
                          hintText: 'Party B (Español)...',
                          isDense: true,
                        ),
                        onSubmitted: (val) => _submitTurn('party_b', val),
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.send, color: Colors.cyanAccent),
                      onPressed: () => _submitTurn('party_b', _partyBController.text),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

