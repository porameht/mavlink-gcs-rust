/// Simple MAVLink drone simulator for testing GCS.
/// Uses mavlink tcpin — GCS connects with tcpout.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mavlink::ardupilotmega::*;
use mavlink::{MavHeader, MavConnection, Message};

fn main() {
    let port = std::env::args().nth(1).unwrap_or("5760".into());
    let conn_str = format!("tcpin:0.0.0.0:{port}");
    println!("Mock drone waiting for GCS on {conn_str}...");
    println!("Connect with: cargo run -- -c tcpout:127.0.0.1:{port}");

    let conn = mavlink::connect::<MavMessage>(&conn_str).expect("bind failed");
    let conn = Arc::new(conn);

    // Wait for first message from GCS to confirm connection
    println!("Listening... (will start sending once GCS connects)");

    let state = Arc::new(Mutex::new(DroneState::default()));

    // Receive thread
    let recv_conn = Arc::clone(&conn);
    let recv_state = Arc::clone(&state);
    std::thread::spawn(move || {
        loop {
            match recv_conn.recv() {
                Ok((_header, msg)) => {
                    match msg {
                        MavMessage::HEARTBEAT(_) => {}
                        MavMessage::COMMAND_LONG(cmd) => {
                            println!("CMD: {:?} p1={} p2={}", cmd.command, cmd.param1, cmd.param2);
                            handle_command(&recv_state, &recv_conn, &cmd);
                        }
                        _ => println!("RX: {}", msg.message_name()),
                    }
                }
                Err(_) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    });

    let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
    let mut tick: u64 = 0;

    loop {
        let s = state.lock().unwrap().clone();

        // Heartbeat 1 Hz
        if tick % 10 == 0 {
            let base_mode = if s.armed {
                MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED | MavModeFlag::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED
            } else {
                MavModeFlag::MAV_MODE_FLAG_CUSTOM_MODE_ENABLED
            };
            let _ = conn.send(&header, &MavMessage::HEARTBEAT(HEARTBEAT_DATA {
                custom_mode: s.mode,
                mavtype: MavType::MAV_TYPE_QUADROTOR,
                autopilot: MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
                base_mode,
                system_status: if s.armed { MavState::MAV_STATE_ACTIVE } else { MavState::MAV_STATE_STANDBY },
                mavlink_version: 3,
            }));
            if tick == 0 { println!("Sending telemetry..."); }
        }

        // GPS 4 Hz
        if tick % 2 == 0 {
            let _ = conn.send(&header, &MavMessage::GLOBAL_POSITION_INT(GLOBAL_POSITION_INT_DATA {
                time_boot_ms: (tick * 100) as u32,
                lat: (s.lat * 1e7) as i32,
                lon: (s.lon * 1e7) as i32,
                alt: (s.alt * 1000.0) as i32,
                relative_alt: (s.alt * 1000.0) as i32,
                vx: 0, vy: 0, vz: 0,
                hdg: (s.heading as u16) * 100,
            }));
        }

        // Attitude 10 Hz
        let _ = conn.send(&header, &MavMessage::ATTITUDE(ATTITUDE_DATA {
            time_boot_ms: (tick * 100) as u32,
            roll: (tick as f32 * 0.01).sin() * 0.05,
            pitch: (tick as f32 * 0.013).sin() * 0.03,
            yaw: (s.heading as f32).to_radians(),
            rollspeed: 0.0,
            pitchspeed: 0.0,
            yawspeed: 0.0,
        }));

        // SYS_STATUS 1 Hz
        if tick % 10 == 0 {
            let _ = conn.send(&header, &MavMessage::SYS_STATUS(SYS_STATUS_DATA {
                voltage_battery: 12400,
                current_battery: 500,
                battery_remaining: 87,
                onboard_control_sensors_present: MavSysStatusSensor::empty(),
                onboard_control_sensors_enabled: MavSysStatusSensor::empty(),
                onboard_control_sensors_health: MavSysStatusSensor::empty(),
                load: 0, drop_rate_comm: 0, errors_comm: 0,
                errors_count1: 0, errors_count2: 0, errors_count3: 0, errors_count4: 0,
            }));
        }

        // GPS_RAW 1 Hz
        if tick % 10 == 0 {
            let _ = conn.send(&header, &MavMessage::GPS_RAW_INT(GPS_RAW_INT_DATA {
                time_usec: tick * 100_000,
                lat: (s.lat * 1e7) as i32,
                lon: (s.lon * 1e7) as i32,
                alt: (s.alt * 1000.0) as i32,
                eph: 150, epv: 200, vel: 0, cog: 0,
                fix_type: GpsFixType::GPS_FIX_TYPE_3D_FIX,
                satellites_visible: 12,
            }));
        }

        tick += 1;
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[derive(Clone)]
struct DroneState {
    armed: bool,
    mode: u32,
    lat: f64,
    lon: f64,
    alt: f32,
    heading: f32,
}

impl Default for DroneState {
    fn default() -> Self {
        Self { armed: false, mode: 0, lat: 13.7563, lon: 100.5018, alt: 0.0, heading: 45.0 }
    }
}

fn handle_command(
    state: &Arc<Mutex<DroneState>>,
    conn: &Arc<Box<dyn MavConnection<MavMessage> + Sync + Send>>,
    cmd: &COMMAND_LONG_DATA,
) {
    let header = MavHeader { system_id: 1, component_id: 1, sequence: 0 };
    let result = match cmd.command {
        MavCmd::MAV_CMD_COMPONENT_ARM_DISARM => {
            let mut s = state.lock().unwrap();
            s.armed = cmd.param1 > 0.5;
            println!("  → {}", if s.armed { "ARMED" } else { "DISARMED" });
            MavResult::MAV_RESULT_ACCEPTED
        }
        MavCmd::MAV_CMD_DO_SET_MODE => {
            let mut s = state.lock().unwrap();
            s.mode = cmd.param2 as u32;
            println!("  → MODE {}", s.mode);
            MavResult::MAV_RESULT_ACCEPTED
        }
        MavCmd::MAV_CMD_NAV_TAKEOFF => {
            let mut s = state.lock().unwrap();
            if s.armed {
                s.alt = cmd.param7;
                println!("  → TAKEOFF to {}m", s.alt);
                MavResult::MAV_RESULT_ACCEPTED
            } else {
                println!("  → REJECTED (not armed)");
                MavResult::MAV_RESULT_DENIED
            }
        }
        MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL => MavResult::MAV_RESULT_ACCEPTED,
        _ => {
            println!("  → UNSUPPORTED {:?}", cmd.command);
            MavResult::MAV_RESULT_UNSUPPORTED
        }
    };
    let _ = conn.send(&header, &MavMessage::COMMAND_ACK(COMMAND_ACK_DATA { command: cmd.command, result }));
}
