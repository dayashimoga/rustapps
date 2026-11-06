import 'package:flutter_test/flutter_test.dart';
import 'package:screen_recorder_studio_mobile/main.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('Screen Recorder Studio'), findsWidgets);
  });
}
