use std::sync::Arc;

use anyhow::{Context, Result};
use mavlink::ardupilotmega::MavMessage;
use mavlink::{MavConnection, MavHeader};

pub type MavConn = Box<dyn MavConnection<MavMessage> + Sync + Send>;

/// Connect to a MAVLink endpoint.
/// Formats: "udpin:0.0.0.0:14550", "serial:/dev/ttyUSB0:57600"
pub fn connect(address: &str) -> Result<Arc<MavConn>> {
    let conn = mavlink::connect::<MavMessage>(address)
        .with_context(|| format!("Failed to connect to {address}"))?;
    Ok(Arc::new(conn))
}

/// Send a MAVLink message with GCS header (sysid=255, compid=0)
pub fn send_msg(conn: &MavConn, msg: &MavMessage) -> Result<()> {
    let header = MavHeader {
        system_id: 255,
        component_id: 0,
        sequence: 0,
    };
    conn.send(&header, msg)?;
    Ok(())
}
