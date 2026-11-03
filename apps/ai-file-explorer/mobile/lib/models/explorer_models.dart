/// Domain data models for AI File Explorer Mobile

class FileItem {
  final String path;
  final String fileName;
  final String? extension;
  final int sizeBytes;
  final bool isDir;
  final int modifiedTimestampSecs;
  final bool isReadonly;

  FileItem({
    required this.path,
    required this.fileName,
    this.extension,
    required this.sizeBytes,
    required this.isDir,
    required this.modifiedTimestampSecs,
    required this.isReadonly,
  });

  factory FileItem.fromJson(Map<String, dynamic> json) {
    return FileItem(
      path: json['path'] as String,
      fileName: json['file_name'] as String,
      extension: json['extension'] as String?,
      sizeBytes: (json['size_bytes'] as num).toInt(),
      isDir: json['is_dir'] as bool,
      modifiedTimestampSecs: (json['modified_timestamp_secs'] as num).toInt(),
      isReadonly: json['is_readonly'] as bool,
    );
  }

  Map<String, dynamic> toJson() => {
        'path': path,
        'file_name': fileName,
        'extension': extension,
        'size_bytes': sizeBytes,
        'is_dir': isDir,
        'modified_timestamp_secs': modifiedTimestampSecs,
        'is_readonly': isReadonly,
      };

  String get humanReadableSize {
    if (isDir) return '--';
    if (sizeBytes < 1024) return '$sizeBytes B';
    if (sizeBytes < 1024 * 1024) {
      return '${(sizeBytes / 1024.0).toStringAsFixed(1)} KB';
    }
    if (sizeBytes < 1024 * 1024 * 1024) {
      return '${(sizeBytes / (1024.0 * 1024.0)).toStringAsFixed(1)} MB';
    }
    return '${(sizeBytes / (1024.0 * 1024.0 * 1024.0)).toStringAsFixed(2)} GB';
  }
}

class DirectoryListing {
  final String currentPath;
  final String? parentPath;
  final List<FileItem> items;
  final int totalCount;
  final int totalSizeBytes;

  DirectoryListing({
    required this.currentPath,
    this.parentPath,
    required this.items,
    required this.totalCount,
    required this.totalSizeBytes,
  });

  factory DirectoryListing.fromJson(Map<String, dynamic> json) {
    return DirectoryListing(
      currentPath: json['current_path'] as String,
      parentPath: json['parent_path'] as String?,
      items: (json['items'] as List<dynamic>?)
              ?.map((e) => FileItem.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
      totalCount: (json['total_count'] as num).toInt(),
      totalSizeBytes: (json['total_size_bytes'] as num).toInt(),
    );
  }

  Map<String, dynamic> toJson() => {
        'current_path': currentPath,
        'parent_path': parentPath,
        'items': items.map((e) => e.toJson()).toList(),
        'total_count': totalCount,
        'total_size_bytes': totalSizeBytes,
      };
}

class FolderStats {
  final String rootPath;
  final int totalFiles;
  final int totalDirs;
  final int totalSizeBytes;
  final Map<String, int> extensionCounts;
  final Map<String, int> extensionSizes;
  final List<FileItem> largestFiles;

  FolderStats({
    required this.rootPath,
    required this.totalFiles,
    required this.totalDirs,
    required this.totalSizeBytes,
    required this.extensionCounts,
    required this.extensionSizes,
    required this.largestFiles,
  });

  factory FolderStats.fromJson(Map<String, dynamic> json) {
    return FolderStats(
      rootPath: json['root_path'] as String,
      totalFiles: (json['total_files'] as num).toInt(),
      totalDirs: (json['total_dirs'] as num).toInt(),
      totalSizeBytes: (json['total_size_bytes'] as num).toInt(),
      extensionCounts: (json['extension_counts'] as Map<String, dynamic>?)?.map(
            (k, v) => MapEntry(k, (v as num).toInt()),
          ) ??
          {},
      extensionSizes: (json['extension_sizes'] as Map<String, dynamic>?)?.map(
            (k, v) => MapEntry(k, (v as num).toInt()),
          ) ??
          {},
      largestFiles: (json['largest_files'] as List<dynamic>?)
              ?.map((e) => FileItem.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
    );
  }
}

class DuplicateGroup {
  final int fileSize;
  final String blake3Hash;
  final List<String> filePaths;
  final int wastedBytes;

  DuplicateGroup({
    required this.fileSize,
    required this.blake3Hash,
    required this.filePaths,
    required this.wastedBytes,
  });

  factory DuplicateGroup.fromJson(Map<String, dynamic> json) {
    return DuplicateGroup(
      fileSize: (json['file_size'] as num).toInt(),
      blake3Hash: json['blake3_hash'] as String,
      filePaths: (json['file_paths'] as List<dynamic>).cast<String>(),
      wastedBytes: (json['wasted_bytes'] as num).toInt(),
    );
  }
}

class DeduplicationSummary {
  final int totalScannedFiles;
  final int totalScannedBytes;
  final List<DuplicateGroup> duplicateGroups;
  final int totalDuplicateFiles;
  final int potentialReclaimedBytes;
  final int elapsedMillis;

  DeduplicationSummary({
    required this.totalScannedFiles,
    required this.totalScannedBytes,
    required this.duplicateGroups,
    required this.totalDuplicateFiles,
    required this.potentialReclaimedBytes,
    required this.elapsedMillis,
  });

  factory DeduplicationSummary.fromJson(Map<String, dynamic> json) {
    return DeduplicationSummary(
      totalScannedFiles: (json['total_scanned_files'] as num).toInt(),
      totalScannedBytes: (json['total_scanned_bytes'] as num).toInt(),
      duplicateGroups: (json['duplicate_groups'] as List<dynamic>?)
              ?.map((e) => DuplicateGroup.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
      totalDuplicateFiles: (json['total_duplicate_files'] as num).toInt(),
      potentialReclaimedBytes: (json['potential_reclaimed_bytes'] as num).toInt(),
      elapsedMillis: (json['elapsed_millis'] as num).toInt(),
    );
  }
}

class DocumentMetadata {
  final String? title;
  final String? author;
  final int? pageCount;
  final int wordCount;
  final String format;
  final String? creationDate;

  DocumentMetadata({
    this.title,
    this.author,
    this.pageCount,
    required this.wordCount,
    required this.format,
    this.creationDate,
  });

  factory DocumentMetadata.fromJson(Map<String, dynamic> json) {
    return DocumentMetadata(
      title: json['title'] as String?,
      author: json['author'] as String?,
      pageCount: (json['page_count'] as num?)?.toInt(),
      wordCount: (json['word_count'] as num).toInt(),
      format: json['format'] as String,
      creationDate: json['creation_date'] as String?,
    );
  }
}

class ExtractedDocument {
  final String path;
  final String fileName;
  final String? extension;
  final DocumentMetadata metadata;
  final String contentText;
  final String? language;
  final int extractedAtSecs;

  ExtractedDocument({
    required this.path,
    required this.fileName,
    this.extension,
    required this.metadata,
    required this.contentText,
    this.language,
    required this.extractedAtSecs,
  });

  factory ExtractedDocument.fromJson(Map<String, dynamic> json) {
    return ExtractedDocument(
      path: json['path'] as String,
      fileName: json['file_name'] as String,
      extension: json['extension'] as String?,
      metadata: DocumentMetadata.fromJson(json['metadata'] as Map<String, dynamic>),
      contentText: json['content_text'] as String,
      language: json['language'] as String?,
      extractedAtSecs: (json['extracted_at_secs'] as num).toInt(),
    );
  }
}

class SemanticSearchResult {
  final String path;
  final String fileName;
  final double score;
  final String snippet;
  final String format;

  SemanticSearchResult({
    required this.path,
    required this.fileName,
    required this.score,
    required this.snippet,
    required this.format,
  });

  factory SemanticSearchResult.fromJson(Map<String, dynamic> json) {
    return SemanticSearchResult(
      path: json['path'] as String,
      fileName: json['file_name'] as String,
      score: (json['score'] as num).toDouble(),
      snippet: json['snippet'] as String,
      format: json['format'] as String,
    );
  }
}

class ContentSearchResult {
  final List<SemanticSearchResult> items;
  final int totalMatches;
  final double executionTimeMs;

  ContentSearchResult({
    required this.items,
    required this.totalMatches,
    required this.executionTimeMs,
  });

  factory ContentSearchResult.fromJson(Map<String, dynamic> json) {
    return ContentSearchResult(
      items: (json['items'] as List<dynamic>?)
              ?.map((e) => SemanticSearchResult.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
      totalMatches: (json['total_matches'] as num).toInt(),
      executionTimeMs: (json['execution_time_ms'] as num).toDouble(),
    );
  }
}

class OcrWord {
  final String text;
  final double confidence;
  final List<int> bbox;

  OcrWord({
    required this.text,
    required this.confidence,
    required this.bbox,
  });

  factory OcrWord.fromJson(Map<String, dynamic> json) {
    return OcrWord(
      text: json['text'] as String,
      confidence: (json['confidence'] as num).toDouble(),
      bbox: (json['bbox'] as List<dynamic>).map((e) => (e as num).toInt()).toList(),
    );
  }
}

class OcrResult {
  final String text;
  final List<OcrWord> words;
  final double confidence;
  final int lineCount;
  final double processingTimeMs;

  OcrResult({
    required this.text,
    required this.words,
    required this.confidence,
    required this.lineCount,
    required this.processingTimeMs,
  });

  factory OcrResult.fromJson(Map<String, dynamic> json) {
    return OcrResult(
      text: json['text'] as String,
      words: (json['words'] as List<dynamic>?)
              ?.map((e) => OcrWord.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
      confidence: (json['confidence'] as num).toDouble(),
      lineCount: (json['line_count'] as num).toInt(),
      processingTimeMs: (json['processing_time_ms'] as num).toDouble(),
    );
  }
}

class SensitiveFinding {
  final String id;
  final String dataType;
  final String severity;
  final String filePath;
  final int lineNumber;
  final String redactedPreview;

  SensitiveFinding({
    required this.id,
    required this.dataType,
    required this.severity,
    required this.filePath,
    required this.lineNumber,
    required this.redactedPreview,
  });

  factory SensitiveFinding.fromJson(Map<String, dynamic> json) {
    return SensitiveFinding(
      id: json['id'] as String,
      dataType: json['data_type'] as String,
      severity: json['severity'] as String,
      filePath: json['file_path'] as String,
      lineNumber: (json['line_number'] as num).toInt(),
      redactedPreview: json['redacted_preview'] as String,
    );
  }
}

class ComplianceScanSummary {
  final int scannedFilesCount;
  final int findingsCount;
  final int criticalCount;
  final int highCount;
  final int mediumCount;
  final List<SensitiveFinding> findings;
  final double scanDurationMs;

  ComplianceScanSummary({
    required this.scannedFilesCount,
    required this.findingsCount,
    required this.criticalCount,
    required this.highCount,
    required this.mediumCount,
    required this.findings,
    required this.scanDurationMs,
  });

  factory ComplianceScanSummary.fromJson(Map<String, dynamic> json) {
    return ComplianceScanSummary(
      scannedFilesCount: (json['scanned_files_count'] as num).toInt(),
      findingsCount: (json['findings_count'] as num).toInt(),
      criticalCount: (json['critical_count'] as num).toInt(),
      highCount: (json['high_count'] as num).toInt(),
      mediumCount: (json['medium_count'] as num).toInt(),
      findings: (json['findings'] as List<dynamic>?)
              ?.map((e) => SensitiveFinding.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
      scanDurationMs: (json['scan_duration_ms'] as num).toDouble(),
    );
  }
}

