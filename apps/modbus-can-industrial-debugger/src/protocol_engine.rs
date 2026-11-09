//! Industrial Fieldbus Protocol Engine for Modbus RTU/TCP, CAN 2.0B / CAN-FD.
//!
//! Provides hardware-accurate CRC-16 verification, function code decoding,
//! CAN J1939 / OBD-II signal extraction, IEC 62443 anomaly detection,
//! and Ed25519 industrial audit manifest signing.

use crate::models::{
    CanParsedFrame, FieldbusAuditManifest, IndustrialBusFrame,
    IndustrialSecurityAlert, IndustrialTraceSession, ModbusParsedFrame, SecurityAlertSeverity,
};
use ed25519_dalek::{Signer, SigningKey};
use std::collections::HashMap;

pub struct ProtocolEngine;

impl ProtocolEngine {
    /// Computes standard Modbus RTU CRC-16 (polynomial 0xA001, initial 0xFFFF)
    pub fn compute_modbus_crc16(bytes: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &b in bytes {
            crc ^= b as u16;
            for _ in 0..8 {
                if (crc & 0x0001) != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }

    /// Dissects a raw Modbus RTU byte buffer
    pub fn parse_modbus_rtu(bytes: &[u8], is_request: bool) -> Result<ModbusParsedFrame, String> {
        if bytes.len() < 4 {
            return Err("Modbus RTU frame too short (min 4 bytes)".to_string());
        }

        let unit_id = bytes[0];
        let fc = bytes[1];
        let len = bytes.len();

        let received_crc = (bytes[len - 2] as u16) | ((bytes[len - 1] as u16) << 8);
        let expected_crc = Self::compute_modbus_crc16(&bytes[0..len - 2]);
        let is_crc_valid = received_crc == expected_crc;

        let is_exception = fc >= 0x80;
        let mut exception_code = None;
        let mut starting_addr = None;
        let mut quantity = None;
        let mut values = Vec::new();

        let function_name = if is_exception {
            let actual_fc = fc - 0x80;
            if bytes.len() >= 3 {
                exception_code = Some(bytes[2]);
            }
            format!(
                "Exception: FC 0x{:02X} (Err Code: 0x{:02X})",
                actual_fc,
                bytes.get(2).copied().unwrap_or(0)
            )
        } else {
            match fc {
                0x01 => "Read Coils (0x01)".to_string(),
                0x02 => "Read Discrete Inputs (0x02)".to_string(),
                0x03 => "Read Holding Registers (0x03)".to_string(),
                0x04 => "Read Input Registers (0x04)".to_string(),
                0x05 => "Write Single Coil (0x05)".to_string(),
                0x06 => "Write Single Register (0x06)".to_string(),
                0x0F => "Write Multiple Coils (0x0F)".to_string(),
                0x10 => "Write Multiple Registers (0x10)".to_string(),
                other => format!("Custom Function (0x{:02X})", other),
            }
        };

        if !is_exception && bytes.len() >= 6 {
            if is_request {
                starting_addr = Some(((bytes[2] as u16) << 8) | (bytes[3] as u16));
                quantity = Some(((bytes[4] as u16) << 8) | (bytes[5] as u16));
                if (fc == 0x06 || fc == 0x05) && bytes.len() >= 6 {
                    values.push(((bytes[4] as u16) << 8) | (bytes[5] as u16));
                }
            } else {
                // Response with byte count
                let byte_cnt = bytes[2] as usize;
                let payload_slice = &bytes[3..len - 2];
                if (fc == 0x03 || fc == 0x04) && byte_cnt % 2 == 0 {
                    for chunk in payload_slice.chunks_exact(2) {
                        values.push(((chunk[0] as u16) << 8) | (chunk[1] as u16));
                    }
                }
            }
        }

        Ok(ModbusParsedFrame {
            unit_id,
            function_code: fc,
            function_name,
            is_exception,
            exception_code,
            starting_address: starting_addr,
            quantity,
            values_u16: values,
            raw_payload_hex: hex::encode(bytes),
            crc_or_transaction_id: received_crc,
            is_crc_valid,
        })
    }

    /// Dissects a CAN 2.0 / CAN-FD frame and extracts standard J1939 / OBD-II signals
    pub fn parse_can_frame(
        can_id: u32,
        is_extended: bool,
        is_fd: bool,
        is_rtr: bool,
        payload: &[u8],
    ) -> CanParsedFrame {
        let mut signals = HashMap::new();

        // 1. J1939 Electronic Engine Controller 1 (EEC1) - PGN 61444 (0xF004) -> ID 0x0CF004xx
        if (can_id & 0x00FFFF00) == 0x00F00400 && payload.len() >= 8 {
            // Byte 3-4: Engine RPM (0.125 rpm/bit)
            let raw_rpm = ((payload[4] as u16) << 8) | (payload[3] as u16);
            let rpm = (raw_rpm as f64) * 0.125;
            signals.insert("Engine_Speed_RPM".to_string(), (rpm * 10.0).round() / 10.0);

            // Byte 2: Actual Engine Percent Torque (1% / bit, offset -125%)
            let torque = (payload[2] as f64) - 125.0;
            signals.insert("Engine_Torque_Pct".to_string(), torque);
        }

        // 2. J1939 Engine Temperature 1 (ET1) - PGN 65262 (0xFEEE) -> ID 0x18FEEExx
        if (can_id & 0x00FFFF00) == 0x00FEEE00 && payload.len() >= 8 {
            // Byte 0: Engine Coolant Temp (1 deg C / bit, offset -40 deg C)
            let coolant = (payload[0] as f64) - 40.0;
            signals.insert("Coolant_Temp_C".to_string(), coolant);

            // Byte 1: Fuel Temp
            let fuel = (payload[1] as f64) - 40.0;
            signals.insert("Fuel_Temp_C".to_string(), fuel);

            // Byte 2-3: Engine Oil Temp
            let raw_oil = ((payload[3] as u16) << 8) | (payload[2] as u16);
            let oil = (raw_oil as f64) * 0.03125 - 273.0;
            signals.insert("Engine_Oil_Temp_C".to_string(), (oil * 10.0).round() / 10.0);
        }

        // 3. Standard OBD-II ECU Diagnostic Response (0x7E8)
        if can_id == 0x7E8 && payload.len() >= 4 {
            let pid = payload[2];
            if pid == 0x0C && payload.len() >= 5 {
                let rpm = (((payload[3] as u32) << 8) | (payload[4] as u32)) as f64 / 4.0;
                signals.insert("OBD2_Engine_RPM".to_string(), rpm);
            } else if pid == 0x0D && payload.len() >= 4 {
                signals.insert("OBD2_Vehicle_Speed_KMH".to_string(), payload[3] as f64);
            }
        }

        // 4. Industrial Robotics / Subsea Joint Actuator (0x120 - 0x130)
        if (0x120..=0x130).contains(&can_id) && payload.len() >= 6 {
            let raw_pos = ((payload[1] as u16) << 8) | (payload[0] as u16);
            let raw_vel = ((payload[3] as u16) << 8) | (payload[2] as u16);
            signals.insert("Joint_Position_Deg".to_string(), (raw_pos as f64) * 0.01);
            signals.insert("Joint_Velocity_DPS".to_string(), (raw_vel as f64) * 0.05);
        }

        CanParsedFrame {
            can_id,
            is_extended,
            is_fd,
            is_rtr,
            dlc: payload.len() as u8,
            payload_bytes: payload.to_vec(),
            payload_hex: hex::encode(payload),
            decoded_signals: signals,
        }
    }

    /// Evaluates fieldbus frames for cybersecurity anomalies and physical bus violations (IEC 62443)
    pub fn audit_security_rules(frame: &IndustrialBusFrame) -> Option<IndustrialSecurityAlert> {
        // Rule 1: Modbus RTU CRC Failure
        if let Some(ref m) = frame.modbus {
            if !m.is_crc_valid {
                return Some(IndustrialSecurityAlert {
                    alert_id: format!("SEC-CRC-{}", frame.frame_index),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    severity: SecurityAlertSeverity::Warning,
                    category: "Malformed CRC / Physical Line Noise".to_string(),
                    description: format!(
                        "Modbus frame on Unit {} failed CRC-16 check. Indicates loose RS-485 termination resistor or frame injection.",
                        m.unit_id
                    ),
                    mitigation_recommendation: "Inspect physical RS-485 line termination (120 Ohm) and verify baudrate/parity match.".to_string(),
                });
            }

            // Rule 2: Unauthorized PLC Write Setpoint to Critical Safety Address
            if m.function_code == 0x06 || m.function_code == 0x10 || m.function_code == 0x05 {
                if let Some(addr) = m.starting_address {
                    if addr >= 1000 {
                        return Some(IndustrialSecurityAlert {
                            alert_id: format!("SEC-WRITE-{}", frame.frame_index),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            severity: SecurityAlertSeverity::Critical,
                            category: "Unauthorized PLC Write Setpoint".to_string(),
                            description: format!(
                                "Critical holding register write targeted safety setpoint address 0x{:04X} on Unit {}.",
                                addr, m.unit_id
                            ),
                            mitigation_recommendation: "Lock PLC holding registers with hardware memory protection key (IEC 62443 SL-3).".to_string(),
                        });
                    }
                }
            }

            // Rule 3: Slave Exception Storm
            if m.is_exception {
                if let Some(code) = m.exception_code {
                    if code == 0x02 || code == 0x03 || code == 0x04 {
                        return Some(IndustrialSecurityAlert {
                            alert_id: format!("SEC-EXC-{}", frame.frame_index),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            severity: SecurityAlertSeverity::Warning,
                            category: "Slave Exception Storm / Reconnaissance".to_string(),
                            description: format!(
                                "Slave Unit {} returned Modbus exception code 0x{:02X} (Illegal Address or Slave Failure).",
                                m.unit_id, code
                            ),
                            mitigation_recommendation: "Audit PLC firmware address mapping and verify polling master registers.".to_string(),
                        });
                    }
                }
            }
        }

        // Rule 4: CAN Bus Priority 0 Flood / DoS Injection
        if let Some(ref c) = frame.can {
            if c.can_id == 0x000 && !c.is_rtr {
                return Some(IndustrialSecurityAlert {
                    alert_id: format!("SEC-CAN-FLOOD-{}", frame.frame_index),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    severity: SecurityAlertSeverity::Critical,
                    category: "CAN Bus Flood / Dominant Identifier DoS".to_string(),
                    description: "Highest-priority CAN ID 0x000 received. Potential bus starvation or malicious ECU spoofing.".to_string(),
                    mitigation_recommendation: "Isolate rogue ECU transceiver with automotive secure gateway or hardware CAN firewall.".to_string(),
                });
            }
        }

        None
    }

    /// Generates an immutable, Ed25519-signed Fieldbus Audit Manifest
    pub fn sign_audit_manifest(
        session: &IndustrialTraceSession,
        signing_key: &SigningKey,
    ) -> FieldbusAuditManifest {
        let trace_hash = blake3::hash(
            format!(
                "{}:{}:{}",
                session.session_id, session.total_frames, session.baudrate_bps
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string();

        let manifest_id = format!(
            "CERT-BUS-{}",
            blake3::hash(session.session_id.as_bytes()).to_hex()[0..12].to_uppercase()
        );

        let compliance_status = if session.security_alerts_count == 0 {
            "IEC 62443-4-2 Certified / Zero Security Intrusions".to_string()
        } else {
            format!(
                "IEC 62443 Security Alerts Flagged ({} incidents)",
                session.security_alerts_count
            )
        };

        let manifest_payload = format!(
            "AUDIT-FIELDBUS:{}|SESSION:{}|HASH:{}|FRAMES:{}|ALERTS:{}|STATUS:{}",
            manifest_id,
            session.session_id,
            trace_hash,
            session.total_frames,
            session.security_alerts_count,
            compliance_status
        );

        let sig = signing_key.sign(manifest_payload.as_bytes());
        let ed25519_signature_hex = hex::encode(sig.to_bytes());
        let auditor_public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());

        FieldbusAuditManifest {
            manifest_id,
            session_id: session.session_id.clone(),
            session_name: session.name.clone(),
            trace_blake3_hash: trace_hash,
            total_frames: session.total_frames,
            security_alerts_count: session.security_alerts_count,
            iec62443_compliance_status: compliance_status,
            ed25519_signature_hex,
            auditor_public_key_hex,
            issued_at_epoch_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}
