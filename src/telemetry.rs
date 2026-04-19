use std::sync::{Arc, Mutex};
use std::time::Instant;

use mavlink::ardupilotmega::{MavMessage, MavModeFlag};

use crate::vehicle::{VehicleState, copter_mode_name};

/// Process a received MAVLink message and update vehicle state
pub fn handle_message(state: &Arc<Mutex<VehicleState>>, header: &mavlink::MavHeader, msg: &MavMessage) {
    let mut s = state.lock().unwrap();

    match msg {
        MavMessage::HEARTBEAT(hb) => {
            s.connected = true;
            s.last_heartbeat = Some(Instant::now());
            s.target_system = header.system_id;

            let new_mode = copter_mode_name(hb.custom_mode);
            if s.mode != new_mode {
                s.mode = new_mode.to_string();
            }
            s.armed = hb.base_mode.contains(MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED);

            let new_status = format!("{:?}", hb.system_status);
            if s.system_status != new_status {
                s.system_status = new_status;
            }
        }

        MavMessage::GLOBAL_POSITION_INT(gps) => {
            s.lat = gps.lat as f64 / 1e7;
            s.lon = gps.lon as f64 / 1e7;
            s.alt = gps.alt as f32 / 1000.0;
            s.relative_alt = gps.relative_alt as f32 / 1000.0;
            s.heading = gps.hdg / 100;
        }

        MavMessage::ATTITUDE(att) => {
            s.roll = att.roll.to_degrees();
            s.pitch = att.pitch.to_degrees();
            s.yaw = att.yaw.to_degrees();
        }

        MavMessage::SYS_STATUS(sys) => {
            s.voltage = sys.voltage_battery as f32 / 1000.0;
            s.battery_remaining = sys.battery_remaining;
        }

        MavMessage::GPS_RAW_INT(gps) => {
            s.fix_type = gps.fix_type as u8;
            s.satellites = gps.satellites_visible;
        }

        MavMessage::PARAM_VALUE(p) => {
            let name = String::from_utf8_lossy(&p.param_id).trim_end_matches('\0').to_string();
            s.log_msg(format!("PARAM: {name} = {}", p.param_value));
        }

        _ => {}
    }
}
