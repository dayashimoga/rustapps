/// High-level AI File Explorer Service for Flutter Mobile over native C-ABI FFI
///
/// Handles JSON marshaling, pointer allocations, memory freeing, and error mapping.
/// Supports Mock Mode for testing without native compiled binaries.

import 'dart:convert';
import 'dart:ffi';
import 'package:ffi/ffi.dart';
import '../bindings/afe_ffi.dart';
import '../models/explorer_models.dart';

class AfeService {
  AfeFfiBindings? _ffi;
  Pointer<AfeHandleOpaque>? _handle;
  bool _mockMode = false;

  // Mock state for tests or standalone mode
  final List<FileItem> _mockFiles = [];

  AfeService({bool mockMode = false}) : _mockMode = mockMode;

  bool get isMockMode => _mockMode;
  bool get isOpen => _handle != null || _mockMode;

  /// Open or initialize AI File Explorer handle
  Future<void> open({String? dataDir}) async {
    if (_mockMode) {
      _initMockData();
      return;
    }

    try {
      _ffi = AfeFfiBindings.load();
      Pointer<Utf8> dirPtr = nullptr;
      if (dataDir != null && dataDir.isNotEmpty) {
        dirPtr = dataDir.toNativeUtf8();
      }

      try {
        _handle = _ffi!.createHandle(dirPtr);
        if (_handle == nullptr) {
          throw Exception('Failed to allocate native AFE handle');
        }
      } finally {
        if (dirPtr != nullptr) {
          calloc.free(dirPtr);
        }
      }
    } catch (_) {
      // Fallback to mock mode if native binary cannot be loaded
      _mockMode = true;
      _initMockData();
    }
  }

  void _initMockData() {
    _mockFiles.clear();
    _mockFiles.addAll([
      FileItem(
        path: '/storage/emulated/0/Documents/project_plan.md',
        fileName: 'project_plan.md',
        extension: 'md',
        sizeBytes: 4096,
        isDir: false,
        modifiedTimestampSecs: 1729300000,
        isReadonly: false,
      ),
      FileItem(
        path: '/storage/emulated/0/DCIM/receipt_scan.png',
        fileName: 'receipt_scan.png',
        extension: 'png',
        sizeBytes: 1048576,
        isDir: false,
        modifiedTimestampSecs: 1729300050,
        isReadonly: false,
      ),
      FileItem(
        path: '/storage/emulated/0/Download',
        fileName: 'Download',
        extension: null,
        sizeBytes: 0,
        isDir: true,
        modifiedTimestampSecs: 1729300100,
        isReadonly: false,
      ),
    ]);
  }

  /// Close and dispose native AFE handle
  void close() {
    if (_handle != null && _ffi != null) {
      _ffi!.freeHandle(_handle!);
      _handle = null;
    }
  }

  /// List single directory contents
  Future<DirectoryListing> listDirectory(String path) async {
    if (_mockMode) {
      return DirectoryListing(
        currentPath: path,
        parentPath: '/storage/emulated/0',
        items: _mockFiles,
        totalCount: _mockFiles.length,
        totalSizeBytes: _mockFiles.fold(0, (acc, item) => acc + item.sizeBytes),
      );
    }

    final pathPtr = path.toNativeUtf8();
    try {
      final resPtr = _ffi!.listDirectory(_handle!, pathPtr);
      return _parseJsonAndFree(resPtr, (j) => DirectoryListing.fromJson(j));
    } finally {
      calloc.free(pathPtr);
    }
  }

  /// Search indexed files (<50ms SLA)
  Future<List<FileItem>> searchFiles(String query, {int limit = 50}) async {
    if (_mockMode) {
      final q = query.toLowerCase();
      return _mockFiles.where((f) => f.fileName.toLowerCase().contains(q)).toList();
    }

    final queryPtr = query.toNativeUtf8();
    try {
      final resPtr = _ffi!.searchFiles(_handle!, queryPtr, limit);
      return _parseJsonListAndFree(resPtr, (j) => FileItem.fromJson(j));
    } finally {
      calloc.free(queryPtr);
    }
  }

  /// Index target directory into Tantivy search index
  Future<int> indexDirectory(String path) async {
    if (_mockMode) {
      return _mockFiles.length;
    }

    final pathPtr = path.toNativeUtf8();
    try {
      final resPtr = _ffi!.indexDirectory(_handle!, pathPtr);
      return _parseJsonAndFree(resPtr, (j) => (j['indexed_count'] as num).toInt());
    } finally {
      calloc.free(pathPtr);
    }
  }

  /// Scan for duplicate files using 3-tier filtration
  Future<DeduplicationSummary> scanDuplicates(
    List<String> roots, {
    int minSizeBytes = 1048576,
  }) async {
    if (_mockMode) {
      return DeduplicationSummary(
        totalScannedFiles: 42,
        totalScannedBytes: 25000000,
        duplicateGroups: [
          DuplicateGroup(
            fileSize: 1048576,
            blake3Hash: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
            filePaths: [
              '/storage/emulated/0/DCIM/receipt_scan.png',
              '/storage/emulated/0/Download/receipt_scan_copy.png'
            ],
            wastedBytes: 1048576,
          ),
        ],
        totalDuplicateFiles: 2,
        potentialReclaimedBytes: 1048576,
        elapsedMillis: 18,
      );
    }

    final rootsJson = jsonEncode(roots).toNativeUtf8();
    try {
      final resPtr = _ffi!.scanDuplicates(_handle!, rootsJson, minSizeBytes);
      return _parseJsonAndFree(resPtr, (j) => DeduplicationSummary.fromJson(j));
    } finally {
      calloc.free(rootsJson);
    }
  }

  /// Analyze directory storage breakdown
  Future<FolderStats> analyzeStorage(String path, {int topLimit = 10}) async {
    if (_mockMode) {
      return FolderStats(
        rootPath: path,
        totalFiles: 42,
        totalDirs: 5,
        totalSizeBytes: 25000000,
        extensionCounts: {'md': 10, 'png': 20, 'pdf': 12},
        extensionSizes: {'md': 50000, 'png': 15000000, 'pdf': 9950000},
        largestFiles: _mockFiles,
      );
    }

    final pathPtr = path.toNativeUtf8();
    try {
      final resPtr = _ffi!.analyzeStorage(_handle!, pathPtr, topLimit);
      return _parseJsonAndFree(resPtr, (j) => FolderStats.fromJson(j));
    } finally {
      calloc.free(pathPtr);
    }
  }

  /// Extract document text and metadata
  Future<ExtractedDocument> extractDocument(String path) async {
    if (_mockMode) {
      return ExtractedDocument(
        path: path,
        fileName: path.split('/').last,
        extension: 'md',
        metadata: DocumentMetadata(
          title: 'Project Specification',
          author: 'Mobile Lead',
          pageCount: 3,
          wordCount: 840,
          format: 'Markdown',
          creationDate: '2026-10-19',
        ),
        contentText: '# Mobile Architecture\nOffline file inspection with zero telemetry.',
        language: 'markdown',
        extractedAtSecs: 1729300000,
      );
    }

    final pathPtr = path.toNativeUtf8();
    try {
      final resPtr = _ffi!.extractDocument(_handle!, pathPtr);
      return _parseJsonAndFree(resPtr, (j) => ExtractedDocument.fromJson(j));
    } finally {
      calloc.free(pathPtr);
    }
  }

  /// Keyword, semantic, or combined hybrid content search
  Future<ContentSearchResult> searchContent(
    String query, {
    String mode = 'Combined',
    int limit = 20,
    String? extensionFilter,
  }) async {
    if (_mockMode) {
      return ContentSearchResult(
        items: [
          SemanticSearchResult(
            path: '/storage/emulated/0/Documents/project_plan.md',
            fileName: 'project_plan.md',
            score: 0.92,
            snippet: 'Offline file inspection with zero telemetry...',
            format: 'md',
          ),
        ],
        totalMatches: 1,
        executionTimeMs: 12.5,
      );
    }

    final qMap = {
      'query': query,
      'mode': mode,
      'limit': limit,
      'extension_filter': extensionFilter,
    };
    final qJson = jsonEncode(qMap).toNativeUtf8();
    try {
      final resPtr = _ffi!.searchContent(_handle!, qJson);
      return _parseJsonAndFree(resPtr, (j) => ContentSearchResult.fromJson(j));
    } finally {
      calloc.free(qJson);
    }
  }

  /// Perform offline OCR on image file
  Future<OcrResult> ocrImage(String imagePath) async {
    if (_mockMode) {
      return OcrResult(
        text: 'Antigravity Zero Mock Engine',
        words: [
          OcrWord(text: 'Antigravity', confidence: 0.98, bbox: [0, 0, 80, 20]),
          OcrWord(text: 'Zero', confidence: 0.95, bbox: [85, 0, 120, 20]),
          OcrWord(text: 'Mock', confidence: 0.96, bbox: [125, 0, 160, 20]),
          OcrWord(text: 'Engine', confidence: 0.99, bbox: [165, 0, 210, 20]),
        ],
        confidence: 0.97,
        lineCount: 1,
        processingTimeMs: 24.5,
      );
    }

    final pathPtr = imagePath.toNativeUtf8();
    try {
      final resPtr = _ffi!.ocrImage(_handle!, pathPtr);
      return _parseJsonAndFree(resPtr, (j) => OcrResult.fromJson(j));
    } finally {
      calloc.free(pathPtr);
    }
  }

  /// Perform Business-tier PII compliance scan
  Future<ComplianceScanSummary> scanCompliance(
    String path, {
    String tier = 'business',
  }) async {
    if (_mockMode) {
      return ComplianceScanSummary(
        scannedFilesCount: 15,
        findingsCount: 2,
        criticalCount: 1,
        highCount: 1,
        mediumCount: 0,
        scanDurationMs: 8.2,
        findings: [
          SensitiveFinding(
            id: 'find-mock-1',
            dataType: 'private_key',
            severity: 'critical',
            filePath: '$path/id_rsa',
            lineNumber: 1,
            redactedPreview: '-----BEGIN RSA PRIVATE KEY-----[REDACTED]-----END RSA PRIVATE KEY-----',
          ),
          SensitiveFinding(
            id: 'find-mock-2',
            dataType: 'ssn',
            severity: 'high',
            filePath: '$path/customer_records.csv',
            lineNumber: 14,
            redactedPreview: 'Record #14: SSN: XXX-XX-9876, City: Austin',
          ),
        ],
      );
    }

    final pathPtr = path.toNativeUtf8();
    final tierPtr = tier.toNativeUtf8();
    try {
      final resPtr = _ffi!.scanCompliance(_handle!, pathPtr, tierPtr);
      return _parseJsonAndFree(resPtr, (j) => ComplianceScanSummary.fromJson(j));
    } finally {
      calloc.free(pathPtr);
      calloc.free(tierPtr);
    }
  }

  // -------------------------------------------------------------------------
  // JSON Deserialization & Pointer Freeing Helpers
  // -------------------------------------------------------------------------

  T _parseJsonAndFree<T>(
    Pointer<Utf8> ptr,
    T Function(Map<String, dynamic>) parser,
  ) {
    if (ptr == nullptr) {
      throw Exception('Null pointer returned from native FFI call');
    }
    try {
      final str = ptr.toDartString();
      final map = jsonDecode(str) as Map<String, dynamic>;
      if (map.containsKey('error')) {
        throw Exception(map['error'] as String);
      }
      return parser(map);
    } finally {
      _ffi!.freeString(ptr);
    }
  }

  List<T> _parseJsonListAndFree<T>(
    Pointer<Utf8> ptr,
    T Function(Map<String, dynamic>) parser,
  ) {
    if (ptr == nullptr) {
      throw Exception('Null pointer returned from native FFI call');
    }
    try {
      final str = ptr.toDartString();
      final decoded = jsonDecode(str);
      if (decoded is Map<String, dynamic> && decoded.containsKey('error')) {
        throw Exception(decoded['error'] as String);
      }
      final list = decoded as List<dynamic>;
      return list.map((e) => parser(e as Map<String, dynamic>)).toList();
    } finally {
      _ffi!.freeString(ptr);
    }
  }
}

