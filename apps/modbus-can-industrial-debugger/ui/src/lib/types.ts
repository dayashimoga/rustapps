export type BusProtocol = 'modbus_rtu' | 'modbus_tcp' | 'can_20a' | 'can_20b' | 'can_fd';

export interface ModbusParsedFrame {
  unit_id: number;
  function_code: number;
  function_name: string;
  is_exception: boolean;
  exception_code?: number;
  register_address?: number;
  register_count?: number;
  register_values: number[];
  crc_valid: boolean;
  expected_crc: number;
  actual_crc: number;
}

export interface CanParsedFrame {
  can_id_hex: string;
  is_extended: boolean;
  is_fd: boolean;
  is_rtr: boolean;
  payload_len: number;
  payload_hex: string;
  decoded_signals: Record<string, string>;
  dlc: number;
}

export type SecurityAlertSeverity = 'info' | 'low' | 'medium' | 'high' | 'critical';

export interface IndustrialSecurityAlert {
  alert_id: string;
  severity: SecurityAlertSeverity;
  title: string;
  description: string;
  iec_62443_rule: string;
  mitigation_guidance: string;
}

export interface IndustrialBusFrame {
  frame_index: number;
  timestamp_micros: number;
  protocol: BusProtocol;
  direction: string;
  modbus?: ModbusParsedFrame;
  can?: CanParsedFrame;
  security_alert?: IndustrialSecurityAlert;
}

export interface IndustrialTraceSession {
  session_id: string;
  name: string;
  protocol: BusProtocol;
  baudrate_bps: number;
  total_frames: number;
  error_frames_count: number;
  security_alerts_count: number;
  frames: IndustrialBusFrame[];
  created_at_epoch_ms: number;
}

export interface SessionSummary {
  session_id: string;
  name: string;
  protocol: BusProtocol;
  baudrate_bps: number;
  total_frames: number;
  security_alerts_count: number;
  created_at_epoch_ms: number;
}

export interface FieldbusAuditManifest {
  manifest_id: string;
  session_id: string;
  session_name: string;
  trace_blake3_hash: string;
  total_frames: number;
  security_alerts_count: number;
  iec62443_compliance_status: string;
  ed25519_signature_hex: string;
  auditor_public_key_hex: string;
  issued_at_epoch_ms: number;
}

export interface DebuggerStats {
  total_traces_captured: number;
  total_frames_decoded: number;
  security_intrusions_blocked: number;
  crc_bus_faults_isolated: number;
}

export interface LicenseStatus {
  tier: string;
  active: boolean;
  days_remaining?: number;
  licensee?: string;
}
