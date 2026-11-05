import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import '../lib/models/explorer_models.dart';
import '../lib/services/afe_service.dart';
import '../lib/screens/browser_screen.dart';
import '../lib/screens/content_search_screen.dart';
import '../lib/screens/compliance_screen.dart';

void main() {
  group('AI File Explorer Mobile Models Unit Tests', () {
    test('FileItem serialization and humanReadableSize calculation', () {
      final json = {
        'path': '/documents/report.pdf',
        'file_name': 'report.pdf',
        'extension': 'pdf',
        'size_bytes': 1048576 * 5, // 5 MB
        'is_dir': false,
        'modified_timestamp_secs': 1729300000,
        'is_readonly': false,
      };

      final item = FileItem.fromJson(json);
      expect(item.path, '/documents/report.pdf');
      expect(item.fileName, 'report.pdf');
      expect(item.extension, 'pdf');
      expect(item.sizeBytes, 5242880);
      expect(item.isDir, false);
      expect(item.humanReadableSize, '5.0 MB');

      final outJson = item.toJson();
      expect(outJson['path'], '/documents/report.pdf');
      expect(outJson['size_bytes'], 5242880);
    });

    test('DirectoryListing parsing', () {
      final json = {
        'current_path': '/storage/emulated/0',
        'parent_path': null,
        'items': [
          {
            'path': '/storage/emulated/0/DCIM',
            'file_name': 'DCIM',
            'extension': null,
            'size_bytes': 0,
            'is_dir': true,
            'modified_timestamp_secs': 1729300000,
            'is_readonly': false,
          },
          {
            'path': '/storage/emulated/0/notes.txt',
            'file_name': 'notes.txt',
            'extension': 'txt',
            'size_bytes': 120,
            'is_dir': false,
            'modified_timestamp_secs': 1729300010,
            'is_readonly': false,
          },
        ],
        'total_count': 2,
        'total_size_bytes': 120,
      };

      final listing = DirectoryListing.fromJson(json);
      expect(listing.currentPath, '/storage/emulated/0');
      expect(listing.parentPath, isNull);
      expect(listing.totalCount, 2);
      expect(listing.items.length, 2);
      expect(listing.items[0].isDir, true);
      expect(listing.items[1].isDir, false);
    });

    test('DeduplicationSummary parsing', () {
      final json = {
        'total_scanned_files': 50,
        'total_scanned_bytes': 10000000,
        'duplicate_groups': [
          {
            'file_size': 1024,
            'blake3_hash': 'abcdef1234567890',
            'file_paths': ['/a/doc.txt', '/b/doc_copy.txt'],
            'wasted_bytes': 1024,
          }
        ],
        'total_duplicate_files': 2,
        'potential_reclaimed_bytes': 1024,
        'elapsed_millis': 25,
      };

      final summary = DeduplicationSummary.fromJson(json);
      expect(summary.totalScannedFiles, 50);
      expect(summary.duplicateGroups.length, 1);
      expect(summary.duplicateGroups[0].filePaths.length, 2);
      expect(summary.potentialReclaimedBytes, 1024);
    });

    test('OcrResult and OcrWord parsing', () {
      final json = {
        'text': 'Hello World',
        'words': [
          {'text': 'Hello', 'confidence': 0.99, 'bbox': [0, 0, 40, 10]},
          {'text': 'World', 'confidence': 0.98, 'bbox': [45, 0, 85, 10]},
        ],
        'confidence': 0.985,
        'line_count': 1,
        'processing_time_ms': 12.3,
      };

      final ocr = OcrResult.fromJson(json);
      expect(ocr.text, 'Hello World');
      expect(ocr.words.length, 2);
      expect(ocr.words[0].text, 'Hello');
      expect(ocr.confidence, 0.985);
      expect(ocr.lineCount, 1);
    });

    test('ComplianceScanSummary and SensitiveFinding parsing', () {
      final json = {
        'scanned_files_count': 12,
        'findings_count': 1,
        'critical_count': 1,
        'high_count': 0,
        'medium_count': 0,
        'findings': [
          {
            'id': 'f-1',
            'data_type': 'private_key',
            'severity': 'critical',
            'file_path': '/keys/id_rsa',
            'line_number': 1,
            'redacted_preview': '-----BEGIN RSA PRIVATE KEY-----',
          }
        ],
        'scan_duration_ms': 5.4,
      };

      final comp = ComplianceScanSummary.fromJson(json);
      expect(comp.scannedFilesCount, 12);
      expect(comp.findingsCount, 1);
      expect(comp.criticalCount, 1);
      expect(comp.findings[0].dataType, 'private_key');
      expect(comp.findings[0].severity, 'critical');
    });

    test('ArchiveEntryItem parsing', () {
      final json = {
        'path': 'docs/manual.pdf',
        'size': 10240,
        'compressed_size': 4096,
        'is_dir': false,
        'crc32': 12345678,
      };
      final item = ArchiveEntryItem.fromJson(json);
      expect(item.path, 'docs/manual.pdf');
      expect(item.size, 10240);
      expect(item.compressedSize, 4096);
      expect(item.isDir, false);
      expect(item.crc32, 12345678);
    });
  });

  group('AfeService Mock Integration Tests', () {
    test('AfeService workflow in mock mode', () async {
      final service = AfeService(mockMode: true);
      await service.open();
      expect(service.isOpen, true);

      // Directory listing
      final listing = await service.listDirectory('/storage/emulated/0');
      expect(listing.items.isNotEmpty, true);

      // Search files
      final searchHits = await service.searchFiles('plan');
      expect(searchHits.isNotEmpty, true);
      expect(searchHits.first.fileName, 'project_plan.md');

      // OCR
      final ocr = await service.ocrImage('/storage/emulated/0/DCIM/receipt_scan.png');
      expect(ocr.text, contains('Antigravity'));

      // Compliance
      final comp = await service.scanCompliance('/storage/emulated/0/Documents');
      expect(comp.findingsCount, 2);
      expect(comp.criticalCount, 1);

      // Hash, Archive & Dispatch
      final hash = await service.computeFileHash('/storage/emulated/0/Documents/project_plan.md');
      expect(hash.length, 64);

      final entries = await service.inspectArchive('/storage/emulated/0/archive.zip');
      expect(entries.length, 3);
      expect(entries.first.path, 'README.md');

      final cmdRes = await service.dispatchCommand('ping');
      expect(cmdRes['status'], 'ok');

      service.close();
    });
  });

  group('Widget Tests for Explorer Screens', () {
    testWidgets('BrowserScreen renders correctly', (WidgetTester tester) async {
      final service = AfeService(mockMode: true);
      await service.open();

      await tester.pumpWidget(
        MaterialApp(
          home: BrowserScreen(service: service),
        ),
      );

      await tester.pumpAndSettle();
      expect(find.byType(ListView), findsOneWidget);
      expect(find.text('project_plan.md'), findsOneWidget);
    });

    testWidgets('ContentSearchScreen renders and performs search',
        (WidgetTester tester) async {
      final service = AfeService(mockMode: true);
      await service.open();

      await tester.pumpWidget(
        MaterialApp(
          home: ContentSearchScreen(service: service),
        ),
      );

      await tester.pumpAndSettle();
      expect(find.byType(TextField), findsOneWidget);

      await tester.enterText(find.byType(TextField), 'offline');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      expect(find.text('project_plan.md'), findsOneWidget);
    });

    testWidgets('ComplianceScreen renders and displays findings',
        (WidgetTester tester) async {
      final service = AfeService(mockMode: true);
      await service.open();

      await tester.pumpWidget(
        MaterialApp(
          home: ComplianceScreen(service: service),
        ),
      );

      await tester.pumpAndSettle();
      expect(find.byType(ElevatedButton), findsOneWidget);

      await tester.tap(find.byType(ElevatedButton));
      await tester.pumpAndSettle();

      expect(find.text('PRIVATE KEY'), findsOneWidget);
      expect(find.text('CRITICAL'), findsOneWidget);
    });
  });
}

