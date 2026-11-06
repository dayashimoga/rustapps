import 'package:flutter_test/flutter_test.dart';
import 'package:personal_knowledge_graph_mobile/main.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('Personal Knowledge Graph'), findsWidgets);
  });
}
