import '../bindings/pkg_ffi.dart';

class PkgService {
  final PkgFfiBindings _bindings;

  PkgService({PkgFfiBindings? bindings})
      : _bindings = bindings ?? PkgFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  dynamic getGraphStats() {
    return _bindings.dispatch('get_graph_stats', {});
  }

  dynamic getGraphVisualization() {
    return _bindings.dispatch('get_graph_visualization', {});
  }

  dynamic getNode(String id) {
    return _bindings.dispatch('get_node', {'id': id});
  }

  dynamic saveNote({
    String? id,
    required String title,
    String content = '',
    String nodeType = 'Note',
    List<String>? tags,
  }) {
    return _bindings.dispatch('save_note', {
      if (id != null) 'id': id,
      'title': title,
      'content': content,
      'node_type': nodeType,
      'tags': tags ?? [],
    });
  }

  dynamic deleteNode(String id) {
    return _bindings.dispatch('delete_node', {'id': id});
  }

  dynamic linkNodes({
    required String sourceId,
    required String targetId,
    String relationType = 'related_to',
    String? label,
  }) {
    return _bindings.dispatch('link_nodes', {
      'source_id': sourceId,
      'target_id': targetId,
      'relation_type': relationType,
      if (label != null) 'label': label,
    });
  }

  dynamic searchKnowledge(String query, {bool semantic = true, int limit = 20}) {
    return _bindings.dispatch('search_knowledge', {
      'query': query,
      'semantic': semantic,
      'limit': limit,
    });
  }

  dynamic getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}

