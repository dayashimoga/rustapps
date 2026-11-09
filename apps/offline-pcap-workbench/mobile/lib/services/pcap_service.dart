import '../bindings/pcap_ffi.dart';

class PcapService {
  final OfflinePcapWorkbenchFfiBindings _bindings;

  PcapService({OfflinePcapWorkbenchFfiBindings? bindings})
      : _bindings = bindings ?? OfflinePcapWorkbenchFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> getCaptureSummary(String captureId) {
    return _bindings.dispatch('get_capture_summary', {'capture_id': captureId});
  }

  Map<String, dynamic> getPackets({
    String captureId = '',
    String filter = '',
    int offset = 0,
    int limit = 50,
  }) {
    return _bindings.dispatch('get_packets', {
      'capture_id': captureId,
      'filter': filter,
      'offset': offset,
      'limit': limit,
    });
  }

  Map<String, dynamic> getPacketDetail(String captureId, int packetIndex) {
    return _bindings.dispatch('get_packet_detail', {
      'capture_id': captureId,
      'packet_index': packetIndex,
    });
  }

  Map<String, dynamic> getTcpFlows(String captureId) {
    return _bindings.dispatch('get_tcp_flows', {'capture_id': captureId});
  }

  Map<String, dynamic> getStreamWaterfall(String captureId, int streamId) {
    return _bindings.dispatch('get_stream_waterfall', {
      'capture_id': captureId,
      'stream_id': streamId,
    });
  }

  Map<String, dynamic> getForensicReport(String captureId) {
    return _bindings.dispatch('get_forensic_report', {'capture_id': captureId});
  }

  Map<String, dynamic> parsePcapFile(String filename, String contentHex) {
    return _bindings.dispatch('parse_pcap_file', {
      'filename': filename,
      'content_hex': contentHex,
    });
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String licenseKey) {
    return _bindings.dispatch('apply_license_key', {'license_key': licenseKey});
  }
}
