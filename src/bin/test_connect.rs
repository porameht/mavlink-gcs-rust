use mavlink::ardupilotmega::MavMessage;
use mavlink::Message;
use std::time::{Duration, Instant};

fn main() {
    let addr = std::env::args().nth(1).unwrap_or("tcpout:127.0.0.1:5760".into());
    println!("Connecting to {addr}...");

    let conn = mavlink::connect::<MavMessage>(&addr).expect("connect failed");
    println!("Connected! Waiting for MAVLink data (SITL may take a while under emulation)...");

    let start = Instant::now();
    let mut count = 0;

    loop {
        if start.elapsed() > Duration::from_secs(60) {
            println!("Timeout after 60s — SITL may not be ready yet.");
            break;
        }

        match conn.recv() {
            Ok((header, msg)) => {
                let name = match &msg {
                    MavMessage::HEARTBEAT(hb) => format!("HEARTBEAT mode={} armed={}", hb.custom_mode, hb.base_mode.bits() & 128 != 0),
                    MavMessage::GLOBAL_POSITION_INT(g) => format!("GPS lat={:.7} lon={:.7} alt={:.1}m", g.lat as f64 / 1e7, g.lon as f64 / 1e7, g.alt as f32 / 1000.0),
                    MavMessage::ATTITUDE(a) => format!("ATT roll={:.1} pitch={:.1} yaw={:.1}", a.roll.to_degrees(), a.pitch.to_degrees(), a.yaw.to_degrees()),
                    MavMessage::SYS_STATUS(s) => format!("SYS bat={:.1}V rem={}%", s.voltage_battery as f32 / 1000.0, s.battery_remaining),
                    MavMessage::GPS_RAW_INT(g) => format!("GPS_RAW fix={} sat={}", g.fix_type as u8, g.satellites_visible),
                    _ => format!("{}", msg.message_name()),
                };
                println!("[{:>3}] sys={} {name}", count, header.system_id);
                count += 1;
                if count >= 30 {
                    break;
                }
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(500));
                if start.elapsed().as_secs() % 10 == 0 {
                    print!(".");
                }
            }
        }
    }

    if count > 0 {
        println!("\nMAVLink communication working! Received {count} messages.");
    }
}
