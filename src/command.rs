use mavlink::ardupilotmega::*;

use crate::connection::{MavConn, send_msg};
use crate::vehicle::copter_mode_number;
use anyhow::{bail, Result};

pub fn parse_and_send(conn: &MavConn, input: &str, target_system: u8) -> Result<String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        bail!("empty command");
    }

    match parts[0].to_lowercase().as_str() {
        "arm" => {
            send_command_long(conn, target_system, MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])?;
            Ok("arm → sent".into())
        }
        "disarm" => {
            send_command_long(conn, target_system, MavCmd::MAV_CMD_COMPONENT_ARM_DISARM, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])?;
            Ok("disarm → sent".into())
        }
        "takeoff" => {
            let alt: f32 = parts.get(1).unwrap_or(&"10").parse().unwrap_or(10.0);
            send_command_long(conn, target_system, MavCmd::MAV_CMD_NAV_TAKEOFF, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, alt])?;
            Ok(format!("takeoff {alt}m → sent"))
        }
        "mode" | "rtl" | "land" => {
            let mode_name = match parts[0].to_lowercase().as_str() {
                "rtl" => "RTL",
                "land" => "LAND",
                _ => parts.get(1).unwrap_or(&"STABILIZE"),
            };
            match copter_mode_number(mode_name) {
                Some(n) => {
                    send_set_mode(conn, target_system, n)?;
                    Ok(format!("mode {mode_name} → sent"))
                }
                None => Ok(format!("unknown mode: {mode_name}")),
            }
        }
        "goto" => {
            if parts.len() < 4 {
                return Ok("usage: goto <lat> <lon> <alt>".into());
            }
            let lat: f64 = parts[1].parse().unwrap_or(0.0);
            let lon: f64 = parts[2].parse().unwrap_or(0.0);
            let alt: f32 = parts[3].parse().unwrap_or(10.0);

            let msg = MavMessage::SET_POSITION_TARGET_GLOBAL_INT(SET_POSITION_TARGET_GLOBAL_INT_DATA {
                time_boot_ms: 0,
                lat_int: (lat * 1e7) as i32,
                lon_int: (lon * 1e7) as i32,
                alt,
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                afx: 0.0,
                afy: 0.0,
                afz: 0.0,
                yaw: 0.0,
                yaw_rate: 0.0,
                // position only: ignore velocity, acceleration, yaw
                type_mask: PositionTargetTypemask::from_bits(0b0000_1111_1111_1000).unwrap_or_default(),
                target_system,
                target_component: 0,
                coordinate_frame: MavFrame::MAV_FRAME_GLOBAL_RELATIVE_ALT_INT,
            });
            send_msg(conn, &msg)?;
            Ok(format!("goto {lat},{lon} alt={alt}m → sent"))
        }
        _ => Ok(format!("unknown command: {}", parts[0])),
    }
}

fn send_set_mode(conn: &MavConn, target_system: u8, mode_num: u32) -> Result<()> {
    send_command_long(conn, target_system, MavCmd::MAV_CMD_DO_SET_MODE, &[1.0, mode_num as f32, 0.0, 0.0, 0.0, 0.0, 0.0])
}

fn send_command_long(conn: &MavConn, target_system: u8, command: MavCmd, params: &[f32; 7]) -> Result<()> {
    let msg = MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
        param1: params[0],
        param2: params[1],
        param3: params[2],
        param4: params[3],
        param5: params[4],
        param6: params[5],
        param7: params[6],
        command,
        target_system,
        target_component: 0,
        confirmation: 0,
    });
    send_msg(conn, &msg)?;
    Ok(())
}
