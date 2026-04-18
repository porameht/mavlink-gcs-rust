# MAVLink Ground Control Station (Rust)

A TUI-based Ground Control Station for monitoring telemetry and commanding drones via MAVLink protocol.

```
┌─ MAVLink GCS ──────────────────────────────────────────┐
│  ┌─ Status ──────────┐  ┌─ Attitude ────────────────┐  │
│  │ Conn: ✓ CONNECTED │  │ Roll:   -2.30°            │  │
│  │ Mode: GUIDED      │  │ Pitch:   1.50°            │  │
│  │ Arm:  ARMED       │  │ Yaw:    45.20°            │  │
│  └────────────────────┘  └───────────────────────────┘  │
│  ┌─ GPS ─────────────┐  ┌─ Battery ─────────────────┐  │
│  │ Lat:  13.7563000  │  │ ██████████████░░░  87%    │  │
│  │ Lon: 100.5018000  │  │ 12.4V  87%                │  │
│  │ Alt:  50.2m       │  └───────────────────────────┘  │
│  └────────────────────┘                                │
│  ┌─ Log ─────────────────────────────────────────────┐  │
│  │ 15:03:22  arm → sent                              │  │
│  │ 15:03:22  ACK: ARM_DISARM → ACCEPTED              │  │
│  │ 15:03:25  takeoff 50m → sent                      │  │
│  └───────────────────────────────────────────────────┘  │
│  > _                                                   │
└────────────────────────────────────────────────────────┘
```

## What is this?

```
┌───────────┐  MAVLink (UDP/TCP/Serial)  ┌──────────────┐
│  Your Mac │ ◄────────────────────────► │    Drone     │
│  GCS app  │     read telemetry         │  (Pixhawk +  │
│  (TUI)    │     send commands          │  ArduPilot)  │
└───────────┘                            └──────────────┘
```

- **GCS** — Software on your computer to monitor and command a drone
- **MAVLink** — Industry-standard lightweight protocol for drone communication
- **Pixhawk** — Flight controller board with onboard sensors (gyro, accel, baro, mag)
- **ArduPilot** — Open-source autopilot firmware that runs on Pixhawk

**No real drone needed** — comes with a built-in mock drone simulator for testing.

## Quick Start

```bash
# Terminal 1: Start mock drone
cargo run --bin mock_drone

# Terminal 2: Run GCS
cargo run -- -c tcpout:127.0.0.1:5760
```

## Commands

Type in the GCS and press Enter:

| Command | Action |
|---------|--------|
| `arm` | Arm motors |
| `disarm` | Disarm motors |
| `takeoff 50` | Take off to 50 meters |
| `mode guided` | Switch to GUIDED mode |
| `mode loiter` | Switch to LOITER mode |
| `goto 13.75 100.50 50` | Fly to coordinates (lat lon alt) |
| `rtl` | Return To Launch |
| `land` | Land |
| `q` / `Esc` | Quit |

Every command is sent as a MAVLink `COMMAND_LONG` and waits for `COMMAND_ACK` confirmation.

## Connection Options

```bash
# Mock drone (testing)
cargo run -- -c tcpout:127.0.0.1:5760

# ArduPilot SITL (UDP)
cargo run -- -c udpin:0.0.0.0:14550

# Real Pixhawk (Serial)
cargo run -- -c serial:/dev/tty.usbserial:57600
```

## Telemetry

Real-time data from the drone:

| Data | MAVLink Message | Rate |
|------|-----------------|------|
| GPS (lat, lon, alt, heading) | `GLOBAL_POSITION_INT` | 4 Hz |
| Attitude (roll, pitch, yaw) | `ATTITUDE` | 10 Hz |
| Battery (voltage, %) | `SYS_STATUS` | 1 Hz |
| GPS fix + satellites | `GPS_RAW_INT` | 1 Hz |
| Flight mode + armed status | `HEARTBEAT` | 1 Hz |

## Project Structure

```
src/
├── main.rs          # CLI args, thread spawning, TUI loop
├── lib.rs           # Public module exports
├── connection.rs    # MAVLink connect + send (UDP/TCP/Serial)
├── telemetry.rs     # Parse MAVLink messages → VehicleState
├── command.rs       # Parse user input → MAVLink commands
├── vehicle.rs       # Shared state + ArduCopter mode mapping
├── ui.rs            # Ratatui TUI rendering
└── bin/
    ├── mock_drone.rs    # Drone simulator for testing
    ├── test_connect.rs  # Connection test
    └── test_commands.rs # Command integration test
```

## Tech Stack

| Crate | Purpose |
|-------|---------|
| `mavlink` | MAVLink codec — ardupilotmega dialect, UDP/TCP/Serial |
| `ratatui` + `crossterm` | Terminal UI — panels, gauges, live updates |
| `clap` | CLI argument parsing |
| `anyhow` | Error handling |

## Architecture

```
main thread                    recv thread              heartbeat thread
    │                              │                         │
    ├─ TUI render (20fps)          ├─ conn.recv()            ├─ send HEARTBEAT
    ├─ handle keyboard             ├─ update VehicleState    │   every 1 sec
    ├─ send commands               ├─ log COMMAND_ACK        │
    ├─ check disconnect            └─ loop                   └─ loop
    └─ loop
                    │
        ┌───────────┴───────────┐
        │  Arc<Mutex<Vehicle>>  │
        │  (shared state)       │
        └───────────────────────┘
```

## Testing

```bash
cargo build          # Build
cargo clippy         # Lint (zero warnings)
cargo run -- --help  # Show options

# Automated integration test
cargo run --bin mock_drone -- 5762 &
cargo run --bin test_commands -- 5762

# Manual TUI test
cargo run --bin mock_drone &
cargo run -- -c tcpout:127.0.0.1:5760
```

## Roadmap

- **Phase 1 (done)**: GCS core — connection, telemetry, commands, TUI
- **Phase 2**: GPS auto-follow — PID controller + `SET_POSITION_TARGET_GLOBAL_INT`
- **Phase 3**: Vision + sensor fusion — YOLO object detection + Kalman filter
