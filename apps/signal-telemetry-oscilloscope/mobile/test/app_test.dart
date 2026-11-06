import 'package:flutter_test/flutter_test.dart';
import 'package:signal_telemetry_oscilloscope_mobile/main.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('Signal Oscilloscope'), findsWidgets);
  });
}
