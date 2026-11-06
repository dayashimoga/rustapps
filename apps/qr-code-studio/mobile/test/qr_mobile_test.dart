import 'package:flutter_test/flutter_test.dart';
import 'package:qr_code_studio_mobile/main.dart';

void main() {
  testWidgets('QR Code Studio Mobile UI smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const QrStudioMobileApp());
    expect(find.text('QR Code Studio'), findsOneWidget);
    expect(find.text('Generate ISO/IEC 18004 QR'), findsOneWidget);
  });
}

