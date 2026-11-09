import '../bindings/scope_ffi.dart';

class ScopeService {
  final ScopeFfiBindings _bindings;

  ScopeService({ScopeFfiBindings? bindings})
      : _bindings = bindings ?? ScopeFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  dynamic getOscilloscopeStats() {
    return _bindings.dispatch('get_oscilloscope_stats', {});
  }

  dynamic listCaptures() {
    return _bindings.dispatch('list_captures', {});
  }

  dynamic getCapture({String sessionId = 'latest'}) {
    return _bindings.dispatch('get_capture', {'session_id': sessionId});
  }

  dynamic acquireCapture({
    String title = 'Mobile Telemetry Acquisition Sweep',
    double sampleRateHz = 50000.0,
    int numSamples = 1000,
    String ch1Type = 'Sine',
    double ch1Freq = 1000.0,
    double ch1Amp = 3.3,
    String ch2Type = 'Square',
    double ch2Freq = 500.0,
    double ch2Amp = 5.0,
  }) {
    return _bindings.dispatch('acquire_capture', {
      'title': title,
      'sample_rate_hz': sampleRateHz,
      'num_samples': numSamples,
      'ch1_type': ch1Type,
      'ch1_freq': ch1Freq,
      'ch1_amp': ch1Amp,
      'ch2_type': ch2Type,
      'ch2_freq': ch2Freq,
      'ch2_amp': ch2Amp,
    });
  }

  dynamic getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}

