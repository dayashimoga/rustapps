//! Service Layer for Air-Gapped Industrial Fieldbus Protocol Debugger.
//!
//! Stores telemetry trace sessions, decodes raw live bus captures,
//! and issues Ed25519-signed industrial compliance audit certificates.

use crate::models::{
    BusProtocol, DebuggerStats, FieldbusAuditManifest, IndustrialBusFrame, IndustrialTraceSession,
    SessionSummary,
};
use crate::protocol_engine::ProtocolEngine;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct DebuggerService {
    sessions: RwLock<HashMap<String, IndustrialTraceSession>>,
    signing_key: SigningKey,
}

impl Default for DebuggerService {
    fn default() -> Self {
        Self::new()
    }
}

impl DebuggerService {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        let service = Self {
            sessions: RwLock::new(HashMap::new()),
            signing_key,
        };

        service.seed_sample_sessions();
        service
    }

    fn seed_sample_sessions(&self) {
        // Session 1: Modbus RTU Industrial SCADA
        let mut modbus_frames = Vec::new();

        // Frame 1: Read Holding Registers Request (Unit 1, FC 03, Addr 0, Qty 2)
        // 01 03 00 00 00 02 C4 0B
        let f1_bytes = vec![0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0xC4, 0x0B];
        let m1 = ProtocolEngine::parse_modbus_rtu(&f1_bytes, true).ok();
        let frame1 = IndustrialBusFrame {
            frame_index: 1,
            timestamp_micros: 1000,
            protocol: BusProtocol::ModbusRtu,
            direction: "Request / TX".to_string(),
            modbus: m1,
            can: None,
            security_alert: None,
        };
        modbus_frames.push(frame1);

        // Frame 2: Read Holding Registers Response (Unit 1, FC 03, ByteCount 4, Vals: 0x0866 [2150 psi], 0x0348 [840 gpm])
        // 01 03 04 08 66 03 48 ... + CRC
        let mut f2_raw = vec![0x01, 0x03, 0x04, 0x08, 0x66, 0x03, 0x48];
        let crc2 = ProtocolEngine::compute_modbus_crc16(&f2_raw);
        f2_raw.push((crc2 & 0xFF) as u8);
        f2_raw.push((crc2 >> 8) as u8);
        let m2 = ProtocolEngine::parse_modbus_rtu(&f2_raw, false).ok();
        let frame2 = IndustrialBusFrame {
            frame_index: 2,
            timestamp_micros: 3500,
            protocol: BusProtocol::ModbusRtu,
            direction: "Response / RX".to_string(),
            modbus: m2,
            can: None,
            security_alert: None,
        };
        modbus_frames.push(frame2);

        // Frame 3: Unauthorized PLC Safety Register Write (Unit 1, FC 06, Addr 0x0400 [1024], Val 9999 [0x270F])
        let mut f3_raw = vec![0x01, 0x06, 0x04, 0x00, 0x27, 0x0F];
        let crc3 = ProtocolEngine::compute_modbus_crc16(&f3_raw);
        f3_raw.push((crc3 & 0xFF) as u8);
        f3_raw.push((crc3 >> 8) as u8);
        let m3 = ProtocolEngine::parse_modbus_rtu(&f3_raw, true).ok();
        let mut frame3 = IndustrialBusFrame {
            frame_index: 3,
            timestamp_micros: 12000,
            protocol: BusProtocol::ModbusRtu,
            direction: "Request / TX".to_string(),
            modbus: m3,
            can: None,
            security_alert: None,
        };
        frame3.security_alert = ProtocolEngine::audit_security_rules(&frame3);
        modbus_frames.push(frame3);

        // Frame 4: Slave Exception Response (Unit 1, FC 0x86, Code 0x02 Illegal Address)
        let mut f4_raw = vec![0x01, 0x86, 0x02];
        let crc4 = ProtocolEngine::compute_modbus_crc16(&f4_raw);
        f4_raw.push((crc4 & 0xFF) as u8);
        f4_raw.push((crc4 >> 8) as u8);
        let m4 = ProtocolEngine::parse_modbus_rtu(&f4_raw, false).ok();
        let mut frame4 = IndustrialBusFrame {
            frame_index: 4,
            timestamp_micros: 14500,
            protocol: BusProtocol::ModbusRtu,
            direction: "Response / RX".to_string(),
            modbus: m4,
            can: None,
            security_alert: None,
        };
        frame4.security_alert = ProtocolEngine::audit_security_rules(&frame4);
        modbus_frames.push(frame4);

        // Frame 5: Corrupted Frame with Malformed CRC (Physical Line Noise)
        let f5_raw = vec![0x01, 0x03, 0x00, 0x01, 0x00, 0x01, 0xDE, 0xAD]; // bad CRC
        let m5 = ProtocolEngine::parse_modbus_rtu(&f5_raw, true).ok();
        let mut frame5 = IndustrialBusFrame {
            frame_index: 5,
            timestamp_micros: 22000,
            protocol: BusProtocol::ModbusRtu,
            direction: "Request / TX".to_string(),
            modbus: m5,
            can: None,
            security_alert: None,
        };
        frame5.security_alert = ProtocolEngine::audit_security_rules(&frame5);
        modbus_frames.push(frame5);

        let s1 = IndustrialTraceSession {
            session_id: "SESSION-MODBUS-FACTORY".to_string(),
            name: "Subsea Pipeline Modbus RTU Supervisory Telemetry".to_string(),
            protocol: BusProtocol::ModbusRtu,
            baudrate_bps: 19200,
            total_frames: modbus_frames.len(),
            error_frames_count: 2,
            security_alerts_count: 3,
            frames: modbus_frames,
            created_at_epoch_ms: chrono::Utc::now().timestamp_millis() - 60000,
        };

        // Session 2: CAN 2.0B / J1939 Telematics
        let mut can_frames = Vec::new();

        // CAN Frame 1: J1939 EEC1 (0x0CF00400) RPM=1850.0, Torque=65%
        // payload: [0xFF, 0xFF, 190 (65%), raw_rpm_lo, raw_rpm_hi, 0xFF, 0xFF, 0xFF]
        // raw_rpm = 1850 / 0.125 = 14800 (0x39D0)
        let p1 = vec![0xFF, 0xFF, 190, 0xD0, 0x39, 0xFF, 0xFF, 0xFF];
        let c1 = ProtocolEngine::parse_can_frame(0x0CF00400, true, false, false, &p1);
        let frame_can1 = IndustrialBusFrame {
            frame_index: 1,
            timestamp_micros: 500,
            protocol: BusProtocol::Can20B,
            direction: "RX".to_string(),
            modbus: None,
            can: Some(c1),
            security_alert: None,
        };
        can_frames.push(frame_can1);

        // CAN Frame 2: J1939 ET1 (0x18FEEE00) Coolant=88 C, Fuel=45 C
        // coolant = 88 + 40 = 128 (0x80), fuel = 45 + 40 = 85 (0x55)
        let p2 = vec![128, 85, 0x00, 0x2A, 0xFF, 0xFF, 0xFF, 0xFF];
        let c2 = ProtocolEngine::parse_can_frame(0x18FEEE00, true, false, false, &p2);
        let frame_can2 = IndustrialBusFrame {
            frame_index: 2,
            timestamp_micros: 1500,
            protocol: BusProtocol::Can20B,
            direction: "RX".to_string(),
            modbus: None,
            can: Some(c2),
            security_alert: None,
        };
        can_frames.push(frame_can2);

        // CAN Frame 3: Industrial Robotic Joint (0x120) Pos=45.0 deg, Vel=10.0 dps
        let p3 = vec![0xB4, 0x11, 0xC8, 0x00, 0x00, 0x00];
        let c3 = ProtocolEngine::parse_can_frame(0x120, false, false, false, &p3);
        let frame_can3 = IndustrialBusFrame {
            frame_index: 3,
            timestamp_micros: 2500,
            protocol: BusProtocol::Can20A,
            direction: "TX".to_string(),
            modbus: None,
            can: Some(c3),
            security_alert: None,
        };
        can_frames.push(frame_can3);

        // CAN Frame 4: Malicious Priority 0 Flood (ID 0x000)
        let p4 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let c4 = ProtocolEngine::parse_can_frame(0x000, false, false, false, &p4);
        let mut frame_can4 = IndustrialBusFrame {
            frame_index: 4,
            timestamp_micros: 3500,
            protocol: BusProtocol::Can20A,
            direction: "RX".to_string(),
            modbus: None,
            can: Some(c4),
            security_alert: None,
        };
        frame_can4.security_alert = ProtocolEngine::audit_security_rules(&frame_can4);
        can_frames.push(frame_can4);

        let s2 = IndustrialTraceSession {
            session_id: "SESSION-CAN-AUTOMOTIVE".to_string(),
            name: "Heavy Machinery J1939 Telematics & CAN Diagnostics".to_string(),
            protocol: BusProtocol::Can20B,
            baudrate_bps: 500000,
            total_frames: can_frames.len(),
            error_frames_count: 1,
            security_alerts_count: 1,
            frames: can_frames,
            created_at_epoch_ms: chrono::Utc::now().timestamp_millis() - 30000,
        };

        if let Ok(mut l) = self.sessions.write() {
            l.insert(s1.session_id.clone(), s1);
            l.insert(s2.session_id.clone(), s2);
        }
    }

    /// Evaluates metrics across all recorded fieldbus traces
    pub fn get_stats(&self) -> DebuggerStats {
        let lock = match self.sessions.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut total_frames = 0;
        let mut total_alerts = 0;
        let mut total_errors = 0;

        for s in lock.values() {
            total_frames += s.total_frames;
            total_alerts += s.security_alerts_count;
            total_errors += s.error_frames_count;
        }

        DebuggerStats {
            total_traces_captured: lock.len(),
            total_frames_decoded: total_frames,
            security_intrusions_blocked: total_alerts,
            crc_bus_faults_isolated: total_errors,
        }
    }

    /// Returns list of summaries for all loaded sessions
    pub fn list_sessions(&self) -> Vec<SessionSummary> {
        let lock = match self.sessions.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let mut list: Vec<SessionSummary> = lock
            .values()
            .map(|s| SessionSummary {
                session_id: s.session_id.clone(),
                name: s.name.clone(),
                protocol: s.protocol,
                baudrate_bps: s.baudrate_bps,
                total_frames: s.total_frames,
                security_alerts_count: s.security_alerts_count,
                created_at_epoch_ms: s.created_at_epoch_ms,
            })
            .collect();

        list.sort_by_key(|b| std::cmp::Reverse(b.created_at_epoch_ms));
        list
    }

    /// Retrieves full trace session by ID
    pub fn get_session(&self, session_id: &str) -> Option<IndustrialTraceSession> {
        let lock = match self.sessions.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if session_id.is_empty() {
            return lock.values().next().cloned();
        }

        lock.get(session_id).cloned()
    }

    /// Decodes a single raw payload hex string into a structured bus frame
    pub fn decode_raw_frame(
        &self,
        protocol: BusProtocol,
        raw_hex: &str,
        is_request: bool,
    ) -> Result<IndustrialBusFrame, String> {
        let bytes =
            hex::decode(raw_hex.replace(' ', "")).map_err(|e| format!("Invalid hex: {}", e))?;

        let (modbus, can) = match protocol {
            BusProtocol::ModbusRtu | BusProtocol::ModbusTcp => {
                let m = ProtocolEngine::parse_modbus_rtu(&bytes, is_request)?;
                (Some(m), None)
            }
            BusProtocol::Can20A | BusProtocol::Can20B | BusProtocol::CanFd => {
                let can_id = 0x18FEEE00;
                let c = ProtocolEngine::parse_can_frame(
                    can_id,
                    true,
                    protocol == BusProtocol::CanFd,
                    false,
                    &bytes,
                );
                (None, Some(c))
            }
        };

        let mut frame = IndustrialBusFrame {
            frame_index: 999,
            timestamp_micros: 0,
            protocol,
            direction: if is_request {
                "Request / TX".to_string()
            } else {
                "Response / RX".to_string()
            },
            modbus,
            can,
            security_alert: None,
        };

        frame.security_alert = ProtocolEngine::audit_security_rules(&frame);
        Ok(frame)
    }

    /// Generates an Ed25519-signed Fieldbus Audit Manifest
    pub fn sign_audit_manifest(&self, session_id: &str) -> Result<FieldbusAuditManifest, String> {
        let lock = match self.sessions.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        let session = if session_id.is_empty() {
            lock.values()
                .next()
                .cloned()
                .ok_or_else(|| "No sessions available".to_string())?
        } else {
            lock.get(session_id)
                .cloned()
                .ok_or_else(|| format!("Session '{}' not found", session_id))?
        };

        Ok(ProtocolEngine::sign_audit_manifest(
            &session,
            &self.signing_key,
        ))
    }
}
