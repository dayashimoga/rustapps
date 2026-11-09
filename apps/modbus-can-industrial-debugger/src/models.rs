//! Data models for Air-Gapped Industrial Fieldbus Protocol Debugger.
//!
//! Models Modbus RTU/TCP frames, CAN 2.0B / CAN-FD packets,
//! J1939/OBD-II signals, IEC 62443 security anomalies, and Ed25519 trace audit manifests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported Industrial Fieldbus Protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusProtocol {
    ModbusRtu,
    ModbusTcp,
    Can20A,
    Can20B,
    CanFd,
}

impl BusProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            BusProtocol::ModbusRtu => "Modbus RTU (RS-485)",
            BusProtocol::ModbusTcp => "Modbus TCP (Ethernet)",
            BusProtocol::Can20A => "CAN 2.0A (Standard 11-bit)",
            BusProtocol::Can20B => "CAN 2.0B (Extended 29-bit)",
            BusProtocol::CanFd => "CAN-FD (Flexible Datarate)",
        }
    }
}

/// Parsed Modbus RTU/TCP Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusParsedFrame {
    pub unit_id: u8,
    pub function_code: u8,
    pub function_name: String,
    pub is_exception: bool,
    pub exception_code: Option<u8>,
    pub starting_address: Option<u16>,
    pub quantity: Option<u16>,
    pub values_u16: Vec<u16>,
    pub raw_payload_hex: String,
    pub crc_or_transaction_id: u16,
    pub is_crc_valid: bool,
}

/// Parsed CAN 2.0B / CAN-FD Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanParsedFrame {
    pub can_id: u32,
    pub is_extended: bool,
    pub is_fd: bool,
    pub is_rtr: bool,
    pub dlc: u8,
    pub payload_bytes: Vec<u8>,
    pub payload_hex: String,
    pub decoded_signals: HashMap<String, f64>,
}

/// Severity Level for Industrial Security Alerts (IEC 62443 / NIST SP 800-82)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityAlertSeverity {
    Info,
    Warning,
    Critical,
}

impl SecurityAlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityAlertSeverity::Info => "INFO",
            SecurityAlertSeverity::Warning => "WARNING",
            SecurityAlertSeverity::Critical => "CRITICAL",
        }
    }
}

/// Industrial Operational Security & Anomaly Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialSecurityAlert {
    pub alert_id: String,
    pub timestamp_ms: i64,
    pub severity: SecurityAlertSeverity,
    pub category: String, // "Unauthorized PLC Write", "CAN Bus Flood / DoS", "Malformed CRC", "Slave Exception Storm"
    pub description: String,
    pub mitigation_recommendation: String,
}

/// Unified Single Fieldbus Trace Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialBusFrame {
    pub frame_index: usize,
    pub timestamp_micros: u64,
    pub protocol: BusProtocol,
    pub direction: String, // "Request / TX" or "Response / RX"
    pub modbus: Option<ModbusParsedFrame>,
    pub can: Option<CanParsedFrame>,
    pub security_alert: Option<IndustrialSecurityAlert>,
}

/// Captured Industrial Telemetry Session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialTraceSession {
    pub session_id: String,
    pub name: String,
    pub protocol: BusProtocol,
    pub baudrate_bps: u32,
    pub total_frames: usize,
    pub error_frames_count: usize,
    pub security_alerts_count: usize,
    pub frames: Vec<IndustrialBusFrame>,
    pub created_at_epoch_ms: i64,
}

/// Session Summary for Navigation List
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub name: String,
    pub protocol: BusProtocol,
    pub baudrate_bps: u32,
    pub total_frames: usize,
    pub security_alerts_count: usize,
    pub created_at_epoch_ms: i64,
}

/// Cryptographically Anchored Industrial Audit Manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldbusAuditManifest {
    pub manifest_id: String,
    pub session_id: String,
    pub session_name: String,
    pub trace_blake3_hash: String,
    pub total_frames: usize,
    pub security_alerts_count: usize,
    pub iec62443_compliance_status: String,
    pub ed25519_signature_hex: String,
    pub auditor_public_key_hex: String,
    pub issued_at_epoch_ms: i64,
}

/// Executive Workbench Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerStats {
    pub total_traces_captured: usize,
    pub total_frames_decoded: usize,
    pub security_intrusions_blocked: usize,
    pub crc_bus_faults_isolated: usize,
}
