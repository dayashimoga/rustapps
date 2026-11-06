import 'package:flutter_test/flutter_test.dart';
import 'package:dfir_triage_kit_mobile/main.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('DFIR Triage Kit'), findsWidgets);
  });
}
