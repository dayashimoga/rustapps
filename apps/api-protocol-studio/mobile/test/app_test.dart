import 'package:flutter_test/flutter_test.dart';
import 'package:api_protocol_studio_mobile/main.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('API Protocol Studio'), findsWidgets);
  });
}
