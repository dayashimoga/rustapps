import '../bindings/synthesizer_ffi.dart';

class LocalDataSynthesizerService {
  final LocalDataSynthesizerFfiBindings _bindings;

  LocalDataSynthesizerService({LocalDataSynthesizerFfiBindings? bindings})
      : _bindings = bindings ?? LocalDataSynthesizerFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> getDatasetConfig({String datasetId = 'ds-fintech-demo'}) {
    return _bindings.dispatch('get_dataset_config', {'dataset_id': datasetId});
  }

  Map<String, dynamic> synthesizeDataset({
    String datasetId = 'ds-fintech-demo',
    int? targetRowsOverride,
  }) {
    return _bindings.dispatch('synthesize_dataset', {
      'dataset_id': datasetId,
      if (targetRowsOverride != null) 'target_rows_override': targetRowsOverride,
    });
  }

  Map<String, dynamic> getTablePreview({
    String datasetId = 'ds-fintech-demo',
    String tableName = 'customers',
    int limit = 25,
  }) {
    return _bindings.dispatch('get_table_preview', {
      'dataset_id': datasetId,
      'table_name': tableName,
      'limit': limit,
    });
  }

  Map<String, dynamic> exportTableCsv({
    String datasetId = 'ds-fintech-demo',
    String tableName = 'customers',
  }) {
    return _bindings.dispatch('export_table_csv', {
      'dataset_id': datasetId,
      'table_name': tableName,
    });
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}
