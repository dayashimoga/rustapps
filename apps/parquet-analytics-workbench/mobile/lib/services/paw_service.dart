import '../bindings/paw_ffi.dart';

class PawService {
  final PawFfiBindings _bindings;

  PawService({PawFfiBindings? bindings})
      : _bindings = bindings ?? PawFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  dynamic getWorkbenchStats() {
    return _bindings.dispatch('get_workbench_stats', {});
  }

  dynamic listDatasets() {
    return _bindings.dispatch('list_datasets', {});
  }

  dynamic getSchema(String datasetId) {
    return _bindings.dispatch('get_schema', {'dataset_id': datasetId});
  }

  dynamic seedDemoDataset() {
    return _bindings.dispatch('seed_demo_dataset', {});
  }

  dynamic queryDataset({
    required String datasetId,
    List<String>? columns,
    int limit = 100,
    int offset = 0,
  }) {
    return _bindings.dispatch('query_dataset', {
      'query': {
        'dataset_id': datasetId,
        'columns': columns ?? [],
        'limit': limit,
        'offset': offset,
      }
    });
  }

  dynamic computeColumnStats(String datasetId, String columnName) {
    return _bindings.dispatch('compute_column_stats', {
      'dataset_id': datasetId,
      'column_name': columnName,
    });
  }

  dynamic getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}

