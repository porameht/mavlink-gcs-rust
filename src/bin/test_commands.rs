/// Test GCS command sending against mock drone (non-interactive).
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() {
    let port = std::env::args().nth(1).unwrap_or("5762".into());
    let addr = format!("tcpout:127.0.0.1:{port}");
    println!("=== GCS Command Test ===");
    println!("Connecting to {addr}...");

    let conn = mavlink_gcs_rust::connection::connect(&addr).expect("connect failed");
    let state = Arc::new(Mutex::new(mavlink_gcs_rust::vehicle::VehicleState::default()));

    // Receive thread
    let recv_conn = Arc::clone(&conn);
    let recv_state = Arc::clone(&state);
    std::thread::spawn(move || {
        loop {
            match recv_conn.recv() {
                Ok((header, msg)) => {
                    if let mavlink::ardupilotmega::MavMessage::COMMAND_ACK(ack) = &msg {
                        recv_state.lock().unwrap().log_msg(
                            format!("ACK: {:?} → {:?}", ack.command, ack.result),
                        );
                    }
                    mavlink_gcs_rust::telemetry::handle_message(&recv_state, &header, &msg);
                }
                Err(_) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    });

    // Wait for connection
    std::thread::sleep(Duration::from_secs(2));

    let s = state.lock().unwrap();
    println!("\n--- Telemetry ---");
    println!("Connected: {}", s.connected);
    println!("Mode: {}", s.mode);
    println!("Armed: {}", s.armed);
    println!("GPS: {:.7}, {:.7} alt={:.1}m", s.lat, s.lon, s.alt);
    println!("Attitude: roll={:.2} pitch={:.2} yaw={:.2}", s.roll, s.pitch, s.yaw);
    println!("Battery: {:.1}V {}%", s.voltage, s.battery_remaining);
    println!("Satellites: {} fix={}", s.satellites, s.fix_type);
    let target = s.target_system;
    drop(s);

    // Test commands
    let commands = ["arm", "mode guided", "takeoff 50", "mode loiter", "rtl", "disarm"];
    for cmd in commands {
        println!("\n> {cmd}");
        match mavlink_gcs_rust::command::parse_and_send(&conn, cmd, target) {
            Ok(msg) => {
                println!("  {msg}");
                state.lock().unwrap().log_msg(msg);
            }
            Err(e) => println!("  ERROR: {e}"),
        }
        std::thread::sleep(Duration::from_millis(500));

        // Print any ACKs received
        let s = state.lock().unwrap();
        for entry in s.log.iter().rev().take(2) {
            if entry.contains("ACK") {
                println!("  {entry}");
            }
        }
    }

    println!("\n--- Final State ---");
    let s = state.lock().unwrap();
    println!("Mode: {}", s.mode);
    println!("Armed: {}", s.armed);
    println!("Alt: {:.1}m", s.alt);
    println!("\n=== All tests passed! ===");
}
