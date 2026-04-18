mod command;
mod connection;
mod telemetry;
mod ui;
mod vehicle;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use mavlink::ardupilotmega::*;

use connection::send_msg;
use vehicle::VehicleState;

#[derive(Parser)]
#[command(name = "mavlink-gcs", about = "MAVLink Ground Control Station")]
struct Args {
    /// Connection string: udpin:0.0.0.0:14550 or serial:/dev/ttyUSB0:57600
    #[arg(short, long, default_value = "udpin:0.0.0.0:14550")]
    connect: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let state = Arc::new(Mutex::new(VehicleState::default()));

    let conn = connection::connect(&args.connect)?;
    state.lock().unwrap().log_msg(format!("Connecting to {}...", args.connect));

    // Receive thread
    let recv_conn = Arc::clone(&conn);
    let recv_state = Arc::clone(&state);
    std::thread::spawn(move || {
        loop {
            match recv_conn.recv() {
                Ok((header, msg)) => {
                    if let MavMessage::COMMAND_ACK(ack) = &msg {
                        recv_state.lock().unwrap().log_msg(
                            format!("ACK: {:?} → {:?}", ack.command, ack.result),
                        );
                    }
                    telemetry::handle_message(&recv_state, &header, &msg);
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    });

    // Heartbeat send thread (1 Hz)
    let hb_conn = Arc::clone(&conn);
    std::thread::spawn(move || {
        let heartbeat = MavMessage::HEARTBEAT(HEARTBEAT_DATA {
            custom_mode: 0,
            mavtype: MavType::MAV_TYPE_GCS,
            autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID,
            base_mode: MavModeFlag::default(),
            system_status: MavState::MAV_STATE_ACTIVE,
            mavlink_version: 3,
        });
        loop {
            let _ = send_msg(&hb_conn, &heartbeat);
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    request_data_streams(&conn, 1)?;

    // TUI loop
    let mut terminal = ratatui::init();
    let mut app = ui::App::new();

    let result = loop {
        terminal.draw(|frame| ui::draw(frame, &state, &app))?;
        app.handle_key_event()?;

        // Disconnect detection (inline, no extra thread)
        {
            let mut s = state.lock().unwrap();
            if let Some(last) = s.last_heartbeat && last.elapsed() > Duration::from_secs(5) {
                s.connected = false;
            }
        }

        // Process commands
        let commands: Vec<String> = app.pending_commands.drain(..).collect();
        for cmd in commands {
            let target = state.lock().unwrap().target_system;
            match command::parse_and_send(&conn, &cmd, target) {
                Ok(msg) => state.lock().unwrap().log_msg(msg),
                Err(e) => state.lock().unwrap().log_msg(format!("ERROR: {e}")),
            }
        }

        if app.should_quit {
            break Ok(());
        }
    };

    ratatui::restore();
    result
}

fn request_data_streams(conn: &connection::MavConn, target_system: u8) -> Result<()> {
    let streams: &[(u32, u32)] = &[
        (33, 4),  // GLOBAL_POSITION_INT at 4 Hz
        (30, 10), // ATTITUDE at 10 Hz
        (1, 1),   // SYS_STATUS at 1 Hz
        (24, 1),  // GPS_RAW_INT at 1 Hz
    ];

    for &(msg_id, rate_hz) in streams {
        let interval_us = if rate_hz > 0 { 1_000_000 / rate_hz } else { 0 };
        let msg = MavMessage::COMMAND_LONG(COMMAND_LONG_DATA {
            param1: msg_id as f32,
            param2: interval_us as f32,
            param3: 0.0,
            param4: 0.0,
            param5: 0.0,
            param6: 0.0,
            param7: 0.0,
            command: MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
            target_system,
            target_component: 0,
            confirmation: 0,
        });
        send_msg(conn, &msg)?;
    }
    Ok(())
}
