import 'package:flutter_test/flutter_test.dart';
import 'package:audio_mastering_studio_mobile/main.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('Audio Mastering Studio'), findsWidgets);
  });
}
