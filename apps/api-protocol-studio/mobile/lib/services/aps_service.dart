import '../bindings/aps_ffi.dart';

class ApsService {
  final ApsFfiBindings _bindings;

  ApsService({ApsFfiBindings? bindings})
      : _bindings = bindings ?? ApsFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  dynamic getStudioStats() {
    return _bindings.dispatch('get_studio_stats', {});
  }

  dynamic listCollections() {
    return _bindings.dispatch('list_collections', {});
  }

  dynamic getCollection(String id) {
    return _bindings.dispatch('get_collection', {'id': id});
  }

  dynamic saveCollection(Map<String, dynamic> collection) {
    return _bindings.dispatch('save_collection', {'collection': collection});
  }

  dynamic listRequests(String collectionId) {
    return _bindings.dispatch('list_requests', {'collection_id': collectionId});
  }

  dynamic getRequest(String id) {
    return _bindings.dispatch('get_request', {'id': id});
  }

  dynamic saveRequest(Map<String, dynamic> request) {
    return _bindings.dispatch('save_request', {'request': request});
  }

  dynamic executeRequest(String requestId, {List<Map<String, dynamic>>? variables}) {
    return _bindings.dispatch('execute_request', {
      'request_id': requestId,
      'variables': variables ?? [],
    });
  }

  dynamic fuzzRequest(String requestId, {String strategy = 'sql_injection', String paramName = 'q'}) {
    return _bindings.dispatch('fuzz_request', {
      'request_id': requestId,
      'strategy': strategy,
      'param_name': paramName,
    });
  }

  dynamic getLatestResponse(String requestId) {
    return _bindings.dispatch('get_latest_response', {'request_id': requestId});
  }

  dynamic getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}

