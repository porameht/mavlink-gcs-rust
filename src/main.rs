mod command;
mod connection;
mod gui;
mod telemetry;
mod vehicle;
mod video;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use mavlink::ardupilotmega::*;

use connection::send_msg;
use vehicle::VehicleState;
use video::VideoFrame;

#[derive(Parser)]
#[command(name = "mavlink-gcs", about = "MAVLink Ground Control Station with Video")]
struct Args {
    /// Connection string: tcpout:127.0.0.1:5760 or serial:/dev/ttyUSB0:57600
    #[arg(short, long, default_value = "tcpout:127.0.0.1:5760")]
    connect: String,

    /// Video UDP port (RTP H.264)
    #[arg(long, default_value = "5600")]
    video_port: u16,

    /// Use test video pattern instead of real stream
    #[arg(long)]
    test_video: bool,

    /// Disable video
    #[arg(long)]
    no_video: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let state = Arc::new(Mutex::new(VehicleState::default()));
    let video_frame: Arc<Mutex<Option<VideoFrame>>> = Arc::new(Mutex::new(None));
    let conn_holder: Arc<Mutex<Option<Arc<connection::MavConn>>>> = Arc::new(Mutex::new(None));

    // Connect MAVLink in background (don't block GUI)
    let connect_str = args.connect.clone();
    let conn_state = Arc::clone(&state);
    let conn_store = Arc::clone(&conn_holder);
    std::thread::spawn(move || {
        conn_state.lock().unwrap().log_msg(format!("Connecting to {connect_str}..."));
        match connection::connect(&connect_str) {
            Ok(conn) => {
                conn_state.lock().unwrap().log_msg("MAVLink connected!".into());

                // Request data streams
                let _ = request_data_streams(&conn, 1);

                // Store connection for GUI to use
                *conn_store.lock().unwrap() = Some(Arc::clone(&conn));

                // Receive loop
                let recv_state = Arc::clone(&conn_state);
                let recv_conn = Arc::clone(&conn);
                std::thread::spawn(move || loop {
                    match recv_conn.recv() {
                        Ok((header, msg)) => {
                            if let MavMessage::COMMAND_ACK(ack) = &msg {
                                recv_state.lock().unwrap().log_msg(
                                    format!("ACK: {:?} → {:?}", ack.command, ack.result),
                                );
                            }
                            telemetry::handle_message(&recv_state, &header, &msg);
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(100)),
                    }
                });

                // Heartbeat loop
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
            }
            Err(e) => {
                conn_state.lock().unwrap().log_msg(format!("Connect failed: {e}"));
            }
        }
    });

    // Disconnect detection
    let dc_state = Arc::clone(&state);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let mut s = dc_state.lock().unwrap();
        if let Some(last) = s.last_heartbeat && last.elapsed() > Duration::from_secs(5) {
            s.connected = false;
        }
    });

    // Start video
    let _pipeline = if !args.no_video {
        if args.test_video {
            state.lock().unwrap().log_msg("Starting test video...".into());
            video::start_test_video(Arc::clone(&video_frame)).ok()
        } else {
            state.lock().unwrap().log_msg(format!("Video: waiting on UDP:{}", args.video_port));
            video::start_video_receiver(args.video_port, Arc::clone(&video_frame)).ok()
        }
    } else {
        None
    };

    // Run GUI (main thread)
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 700.0])
            .with_title("MAVLink GCS"),
        ..Default::default()
    };

    let gui_state = Arc::clone(&state);
    let gui_video = Arc::clone(&video_frame);
    let gui_conn = Arc::clone(&conn_holder);

    eframe::run_native(
        "MAVLink GCS",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(gui::GcsApp::new(gui_state, gui_video, gui_conn)))
        }),
    ).map_err(|e| anyhow::anyhow!("GUI error: {e}"))?;

    Ok(())
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
            param3: 0.0, param4: 0.0, param5: 0.0, param6: 0.0, param7: 0.0,
            command: MavCmd::MAV_CMD_SET_MESSAGE_INTERVAL,
            target_system,
            target_component: 0,
            confirmation: 0,
        });
        send_msg(conn, &msg)?;
    }
    Ok(())
}
