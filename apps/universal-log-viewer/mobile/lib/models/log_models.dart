/// Data models for Universal Log Viewer Mobile
///
/// Serde-compatible JSON serialization and deserialization for FFI and UI layers.

class LogLineRecord {
  final int lineNumber;
  final int byteStart;
  final int byteEnd;
  final String text;

  const LogLineRecord({
    required this.lineNumber,
    required this.byteStart,
    required this.byteEnd,
    required this.text,
  });

  factory LogLineRecord.fromJson(Map<String, dynamic> json) {
    return LogLineRecord(
      lineNumber: json['line_number'] as int? ?? 0,
      byteStart: json['byte_start'] as int? ?? 0,
      byteEnd: json['byte_end'] as int? ?? 0,
      text: json['text'] as String? ?? '',
    );
  }

  Map<String, dynamic> toJson() => {
        'line_number': lineNumber,
        'byte_start': byteStart,
        'byte_end': byteEnd,
        'text': text,
      };
}

class OpenedFileInfo {
  final String path;
  final int totalLines;
  final int fileSizeBytes;

  const OpenedFileInfo({
    required this.path,
    required this.totalLines,
    required this.fileSizeBytes,
  });

  factory OpenedFileInfo.fromJson(Map<String, dynamic> json) {
    return OpenedFileInfo(
      path: json['path'] as String? ?? '',
      totalLines: json['total_lines'] as int? ?? 0,
      fileSizeBytes: json['file_size_bytes'] as int? ?? 0,
    );
  }

  Map<String, dynamic> toJson() => {
        'path': path,
        'total_lines': totalLines,
        'file_size_bytes': fileSizeBytes,
      };

  String get filePath => path;
}

class LogSearchMatch {
  final int lineNumber;
  final int byteStart;
  final int byteEnd;
  final String lineContent;
  final int matchStartInLine;
  final int matchEndInLine;

  const LogSearchMatch({
    required this.lineNumber,
    required this.byteStart,
    required this.byteEnd,
    required this.lineContent,
    required this.matchStartInLine,
    required this.matchEndInLine,
  });

  factory LogSearchMatch.fromJson(Map<String, dynamic> json) {
    return LogSearchMatch(
      lineNumber: json['line_number'] as int? ?? 0,
      byteStart: json['byte_start'] as int? ?? 0,
      byteEnd: json['byte_end'] as int? ?? 0,
      lineContent: json['line_content'] as String? ?? '',
      matchStartInLine: json['match_start_in_line'] as int? ?? 0,
      matchEndInLine: json['match_end_in_line'] as int? ?? 0,
    );
  }
}

class LogSearchReport {
  final List<LogSearchMatch> matches;
  final int totalMatchedInScan;
  final int elapsedMicros;
  final bool reachedLimit;

  const LogSearchReport({
    required this.matches,
    required this.totalMatchedInScan,
    required this.elapsedMicros,
    required this.reachedLimit,
  });

  factory LogSearchReport.fromJson(Map<String, dynamic> json) {
    final rawMatches = json['matches'] as List<dynamic>? ?? [];
    return LogSearchReport(
      matches: rawMatches
          .map((m) => LogSearchMatch.fromJson(m as Map<String, dynamic>))
          .toList(),
      totalMatchedInScan: json['total_matched_in_scan'] as int? ?? 0,
      elapsedMicros: json['elapsed_micros'] as int? ?? 0,
      reachedLimit: json['reached_limit'] as bool? ?? false,
    );
  }
}

class TimelineBucket {
  final int bucketIndex;
  final int startEpochMs;
  final int endEpochMs;
  final int totalCount;
  final int traceCount;
  final int debugCount;
  final int infoCount;
  final int warnCount;
  final int errorCount;
  final int fatalCount;
  final double volumePerSecond;

  const TimelineBucket({
    required this.bucketIndex,
    required this.startEpochMs,
    required this.endEpochMs,
    required this.totalCount,
    required this.traceCount,
    required this.debugCount,
    required this.infoCount,
    required this.warnCount,
    required this.errorCount,
    required this.fatalCount,
    required this.volumePerSecond,
  });

  factory TimelineBucket.fromJson(Map<String, dynamic> json) {
    return TimelineBucket(
      bucketIndex: json['bucket_index'] as int? ?? 0,
      startEpochMs: json['start_epoch_ms'] as int? ?? 0,
      endEpochMs: json['end_epoch_ms'] as int? ?? 0,
      totalCount: json['total_count'] as int? ?? 0,
      traceCount: json['trace_count'] as int? ?? 0,
      debugCount: json['debug_count'] as int? ?? 0,
      infoCount: json['info_count'] as int? ?? 0,
      warnCount: json['warn_count'] as int? ?? 0,
      errorCount: json['error_count'] as int? ?? 0,
      fatalCount: json['fatal_count'] as int? ?? 0,
      volumePerSecond: (json['volume_per_second'] as num?)?.toDouble() ?? 0.0,
    );
  }
}

class TimelineHistogram {
  final int minEpochMs;
  final int maxEpochMs;
  final int totalRecords;
  final int bucketWidthMs;
  final List<TimelineBucket> buckets;

  const TimelineHistogram({
    required this.minEpochMs,
    required this.maxEpochMs,
    required this.totalRecords,
    required this.bucketWidthMs,
    required this.buckets,
  });

  factory TimelineHistogram.fromJson(Map<String, dynamic> json) {
    final rawBuckets = json['buckets'] as List<dynamic>? ?? [];
    return TimelineHistogram(
      minEpochMs: json['min_epoch_ms'] as int? ?? 0,
      maxEpochMs: json['max_epoch_ms'] as int? ?? 0,
      totalRecords: json['total_records'] as int? ?? 0,
      bucketWidthMs: json['bucket_width_ms'] as int? ?? 0,
      buckets: rawBuckets
          .map((b) => TimelineBucket.fromJson(b as Map<String, dynamic>))
          .toList(),
    );
  }
}

class LogTemplateCluster {
  final int clusterId;
  final String template;
  final int tokenCount;
  final int frequency;
  final List<int> sampleLines;

  const LogTemplateCluster({
    required this.clusterId,
    required this.template,
    required this.tokenCount,
    required this.frequency,
    required this.sampleLines,
  });

  factory LogTemplateCluster.fromJson(Map<String, dynamic> json) {
    final rawLines = json['sample_lines'] as List<dynamic>? ?? [];
    return LogTemplateCluster(
      clusterId: json['cluster_id'] as int? ?? 0,
      template: json['template'] as String? ?? '',
      tokenCount: json['token_count'] as int? ?? 0,
      frequency: json['frequency'] as int? ?? 0,
      sampleLines: rawLines.map((e) => e as int).toList(),
    );
  }
}

class AnomalySpike {
  final int spikeId;
  final int startEpochMs;
  final int endEpochMs;
  final double zScore;
  final double observedValue;
  final double expectedValue;
  final String metricName;
  final String description;
  final String severity;

  const AnomalySpike({
    required this.spikeId,
    required this.startEpochMs,
    required this.endEpochMs,
    required this.zScore,
    required this.observedValue,
    required this.expectedValue,
    required this.metricName,
    required this.description,
    required this.severity,
  });

  factory AnomalySpike.fromJson(Map<String, dynamic> json) {
    return AnomalySpike(
      spikeId: json['spike_id'] as int? ?? 0,
      startEpochMs: json['start_epoch_ms'] as int? ?? 0,
      endEpochMs: json['end_epoch_ms'] as int? ?? 0,
      zScore: (json['z_score'] as num?)?.toDouble() ?? 0.0,
      observedValue: (json['observed_value'] as num?)?.toDouble() ?? 0.0,
      expectedValue: (json['expected_value'] as num?)?.toDouble() ?? 0.0,
      metricName: json['metric_name'] as String? ?? '',
      description: json['description'] as String? ?? '',
      severity: json['severity']?.toString() ?? 'WARN',
    );
  }
}

class ErrorCluster {
  final int clusterId;
  final String template;
  final int errorCount;
  final List<String> affectedSources;
  final List<int> sampleLines;

  const ErrorCluster({
    required this.clusterId,
    required this.template,
    required this.errorCount,
    required this.affectedSources,
    required this.sampleLines,
  });

  factory ErrorCluster.fromJson(Map<String, dynamic> json) {
    final rawSources = json['affected_sources'] as List<dynamic>? ?? [];
    final rawLines = json['sample_lines'] as List<dynamic>? ?? [];
    return ErrorCluster(
      clusterId: json['cluster_id'] as int? ?? 0,
      template: json['template'] as String? ?? '',
      errorCount: json['error_count'] as int? ?? 0,
      affectedSources: rawSources.map((e) => e.toString()).toList(),
      sampleLines: rawLines.map((e) => e as int).toList(),
    );
  }
}

