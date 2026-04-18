use std::collections::VecDeque;
use std::time::{Instant, SystemTime};

const COPTER_MODES: &[(u32, &str)] = &[
    (0, "STABILIZE"),
    (1, "ACRO"),
    (2, "ALT_HOLD"),
    (3, "AUTO"),
    (4, "GUIDED"),
    (5, "LOITER"),
    (6, "RTL"),
    (7, "CIRCLE"),
    (9, "LAND"),
    (11, "DRIFT"),
    (13, "SPORT"),
    (14, "FLIP"),
    (15, "AUTOTUNE"),
    (16, "POSHOLD"),
    (17, "BRAKE"),
    (18, "THROW"),
    (19, "AVOID_ADSB"),
    (20, "GUIDED_NOGPS"),
    (21, "SMART_RTL"),
    (22, "FLOWHOLD"),
    (23, "FOLLOW"),
    (24, "ZIGZAG"),
    (25, "SYSTEMID"),
    (26, "AUTOROTATE"),
    (27, "AUTO_RTL"),
];

pub fn copter_mode_name(custom_mode: u32) -> &'static str {
    COPTER_MODES
        .iter()
        .find(|(id, _)| *id == custom_mode)
        .map(|(_, name)| *name)
        .unwrap_or("UNKNOWN")
}

pub fn copter_mode_number(name: &str) -> Option<u32> {
    let upper = name.to_uppercase();
    COPTER_MODES
        .iter()
        .find(|(_, n)| *n == upper)
        .map(|(id, _)| *id)
}

pub fn fix_type_name(fix_type: u8) -> &'static str {
    match fix_type {
        0 => "No GPS",
        1 => "No Fix",
        2 => "2D Fix",
        3 => "3D Fix",
        4 => "DGPS",
        5 => "RTK Float",
        6 => "RTK Fixed",
        _ => "Unknown",
    }
}

#[derive(Debug, Clone)]
pub struct VehicleState {
    pub connected: bool,
    pub last_heartbeat: Option<Instant>,
    pub target_system: u8,

    pub armed: bool,
    pub mode: String,
    pub system_status: String,

    pub lat: f64,
    pub lon: f64,
    pub alt: f32,
    pub relative_alt: f32,
    pub heading: u16,
    pub satellites: u8,
    pub fix_type: u8,

    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,

    pub voltage: f32,
    pub battery_remaining: i8,

    pub log: VecDeque<String>,
}

impl Default for VehicleState {
    fn default() -> Self {
        Self {
            connected: false,
            last_heartbeat: None,
            target_system: 1,
            armed: false,
            mode: "UNKNOWN".into(),
            system_status: "UNKNOWN".into(),
            lat: 0.0,
            lon: 0.0,
            alt: 0.0,
            relative_alt: 0.0,
            heading: 0,
            satellites: 0,
            fix_type: 0,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            voltage: 0.0,
            battery_remaining: -1,
            log: VecDeque::new(),
        }
    }
}

impl VehicleState {
    pub fn log_msg(&mut self, msg: String) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs() % 86400;
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        self.log.push_back(format!("{h:02}:{m:02}:{s:02}  {msg}"));
        if self.log.len() > 100 {
            self.log.pop_front();
        }
    }
}
