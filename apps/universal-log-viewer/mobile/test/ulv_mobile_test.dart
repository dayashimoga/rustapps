import 'package:flutter_test/flutter_test.dart';
import '../lib/models/log_models.dart';
import '../lib/services/ulv_service.dart';

void main() {
  group('Universal Log Viewer Mobile Models Unit Tests', () {
    test('LogLineRecord serialization and deserialization', () {
      final json = {
        'line_number': 42,
        'byte_start': 1024,
        'byte_end': 1100,
        'text': '2026-11-03T10:00:00Z [INFO] System running nominally',
      };

      final record = LogLineRecord.fromJson(json);
      expect(record.lineNumber, 42);
      expect(record.byteStart, 1024);
      expect(record.byteEnd, 1100);
      expect(record.text, contains('[INFO]'));

      final out = record.toJson();
      expect(out['line_number'], 42);
      expect(out['text'], record.text);
    });

    test('OpenedFileInfo serialization and deserialization', () {
      final json = {
        'path': '/var/log/app.log',
        'total_lines': 500000,
        'file_size_bytes': 42000000,
      };

      final info = OpenedFileInfo.fromJson(json);
      expect(info.path, '/var/log/app.log');
      expect(info.totalLines, 500000);
      expect(info.fileSizeBytes, 42000000);

      final out = info.toJson();
      expect(out['total_lines'], 500000);
    });

    test('LogSearchReport and LogSearchMatch parsing', () {
      final json = {
        'matches': [
          {
            'line_number': 10,
            'byte_start': 800,
            'byte_end': 860,
            'line_content': 'Timeout acquiring DB lock',
            'match_start_in_line': 0,
            'match_end_in_line': 7,
          }
        ],
        'total_matched_in_scan': 1,
        'elapsed_micros': 340,
        'reached_limit': false,
      };

      final report = LogSearchReport.fromJson(json);
      expect(report.matches.length, 1);
      expect(report.matches[0].lineNumber, 10);
      expect(report.matches[0].lineContent, 'Timeout acquiring DB lock');
      expect(report.totalMatchedInScan, 1);
      expect(report.elapsedMicros, 340);
      expect(report.reachedLimit, isFalse);
    });

    test('TimelineHistogram and TimelineBucket parsing', () {
      final json = {
        'min_epoch_ms': 1700000000000,
        'max_epoch_ms': 1700000060000,
        'total_records': 1000,
        'bucket_width_ms': 1000,
        'buckets': [
          {
            'bucket_index': 0,
            'start_epoch_ms': 1700000000000,
            'end_epoch_ms': 1700000001000,
            'total_count': 50,
            'trace_count': 0,
            'debug_count': 5,
            'info_count': 40,
            'warn_count': 3,
            'error_count': 2,
            'fatal_count': 0,
            'volume_per_second': 50.0,
          }
        ],
      };

      final hist = TimelineHistogram.fromJson(json);
      expect(hist.minEpochMs, 1700000000000);
      expect(hist.buckets.length, 1);
      expect(hist.buckets[0].totalCount, 50);
      expect(hist.buckets[0].errorCount, 2);
      expect(hist.buckets[0].volumePerSecond, 50.0);
    });

    test('LogTemplateCluster parsing', () {
      final json = {
        'cluster_id': 1,
        'template': 'Connection from <IP> closed',
        'token_count': 4,
        'frequency': 150,
        'sample_lines': [1, 5, 9],
      };

      final cluster = LogTemplateCluster.fromJson(json);
      expect(cluster.clusterId, 1);
      expect(cluster.template, 'Connection from <IP> closed');
      expect(cluster.tokenCount, 4);
      expect(cluster.frequency, 150);
      expect(cluster.sampleLines, [1, 5, 9]);
    });

    test('ErrorCluster parsing', () {
      final json = {
        'cluster_id': 2,
        'template': 'Database deadlocked on table <ID>',
        'error_count': 18,
        'affected_sources': ['postgres-pool', 'auth-worker'],
        'sample_lines': [12, 14],
      };

      final ec = ErrorCluster.fromJson(json);
      expect(ec.clusterId, 2);
      expect(ec.template, 'Database deadlocked on table <ID>');
      expect(ec.errorCount, 18);
      expect(ec.affectedSources, contains('postgres-pool'));
      expect(ec.sampleLines, [12, 14]);
    });
  });

  group('UlvService Unit Tests (Mock Mode)', () {
    late UlvService service;

    setUp(() {
      service = UlvService(mockMode: true);
    });

    tearDown(() {
      service.close();
    });

    test('open initializes mock state correctly', () async {
      final info = await service.open('test_file.log');
      expect(info.path, 'test_file.log');
      expect(info.totalLines, 1000);
      expect(service.isOpen, isTrue);
    });

    test('getLinesSlice returns requested lines', () async {
      await service.open('test_file.log');
      final slice = await service.getLinesSlice(10, 5);
      expect(slice.length, 5);
      expect(slice.first.lineNumber, 10);
      expect(slice.last.lineNumber, 14);
    });

    test('search filters lines by pattern', () async {
      await service.open('test_file.log');
      final report = await service.search('Event #12', caseSensitive: false);
      expect(report.matches.isNotEmpty, isTrue);
      expect(report.matches.first.lineContent, contains('Event #12'));
    });

    test('getTimelineHistogram generates time buckets', () async {
      await service.open('test_file.log');
      final hist = await service.getTimelineHistogram(targetBuckets: 20);
      expect(hist.buckets.length, 20);
      expect(hist.totalRecords, 1000);
    });

    test('mineTemplates returns Drain clusters', () async {
      await service.open('test_file.log');
      final templates = await service.mineTemplates();
      expect(templates.isNotEmpty, isTrue);
      expect(templates.first.clusterId, 1);
    });

    test('clusterErrors returns error groups', () async {
      await service.open('test_file.log');
      final clusters = await service.clusterErrors();
      expect(clusters.isNotEmpty, isTrue);
      expect(clusters.first.errorCount, 45);
    });
  });
}

