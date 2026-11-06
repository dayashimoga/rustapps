import 'package:flutter_test/flutter_test.dart';
import 'package:parquet_analytics_workbench_mobile/main.dart';

void main() {
  testWidgets('App smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    expect(find.text('Parquet Analytics Workbench'), findsWidgets);
  });
}
