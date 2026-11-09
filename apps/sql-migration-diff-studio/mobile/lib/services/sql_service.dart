import '../bindings/sql_ffi.dart';

class SqlService {
  final SqlMigrationDiffStudioFfiBindings _bindings;

  SqlService({SqlMigrationDiffStudioFfiBindings? bindings})
      : _bindings = bindings ?? SqlMigrationDiffStudioFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> listSchemas() {
    return _bindings.dispatch('list_schemas', {});
  }

  Map<String, dynamic> getSchema(String schemaId) {
    return _bindings.dispatch('get_schema', {'schema_id': schemaId});
  }

  Map<String, dynamic> parseSchema({
    required String title,
    required String rawDdl,
    String dialect = 'postgresql',
  }) {
    return _bindings.dispatch('parse_schema', {
      'title': title,
      'raw_ddl': rawDdl,
      'dialect': dialect,
    });
  }

  Map<String, dynamic> compareSchemas({
    required String baseId,
    required String targetId,
  }) {
    return _bindings.dispatch('compare_schemas', {
      'base_id': baseId,
      'target_id': targetId,
    });
  }

  Map<String, dynamic> getMigrationPlan(String planId) {
    return _bindings.dispatch('get_migration_plan', {'plan_id': planId});
  }

  Map<String, dynamic> generateIntegrityManifest(String planId) {
    return _bindings.dispatch('generate_integrity_manifest', {
      'plan_id': planId,
    });
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String key) {
    return _bindings.dispatch('apply_license_key', {'license_key': key});
  }
}
