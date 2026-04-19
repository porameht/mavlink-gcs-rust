use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::vehicle::VehicleState;
use crate::video::VideoFrame;

pub struct GcsApp {
    pub state: Arc<Mutex<VehicleState>>,
    pub video_frame: Arc<Mutex<Option<VideoFrame>>>,
    pub conn_holder: Arc<Mutex<Option<Arc<crate::connection::MavConn>>>>,
    pub input: String,
    pub video_texture: Option<egui::TextureHandle>,
}

impl GcsApp {
    pub fn new(
        state: Arc<Mutex<VehicleState>>,
        video_frame: Arc<Mutex<Option<VideoFrame>>>,
        conn_holder: Arc<Mutex<Option<Arc<crate::connection::MavConn>>>>,
    ) -> Self {
        Self {
            state,
            video_frame,
            conn_holder,
            input: String::new(),
            video_texture: None,
        }
    }

    fn send_command(&mut self, cmd: &str) {
        let conn = self.conn_holder.lock().unwrap().clone();
        if let Some(conn) = conn {
            let target = self.state.lock().unwrap().target_system;
            match crate::command::parse_and_send(&conn, cmd, target) {
                Ok(msg) => self.state.lock().unwrap().log_msg(msg),
                Err(e) => self.state.lock().unwrap().log_msg(format!("ERROR: {e}")),
            }
        } else {
            self.state.lock().unwrap().log_msg("Not connected".into());
        }
    }

    fn update_video_texture(&mut self, ctx: &egui::Context) {
        if let Ok(frame_lock) = self.video_frame.lock()
            && let Some(frame) = frame_lock.as_ref()
        {
            let image = egui::ColorImage::from_rgb(
                [frame.width as usize, frame.height as usize],
                &frame.data,
            );
            match &mut self.video_texture {
                Some(tex) => tex.set(image, egui::TextureOptions::LINEAR),
                None => {
                    self.video_texture = Some(ctx.load_texture(
                        "video",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
        }
    }
}

impl eframe::App for GcsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_video_texture(ctx);
        let s = self.state.lock().unwrap().clone();

        // Top panel — status bar
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let conn_color = if s.connected { egui::Color32::GREEN } else { egui::Color32::RED };
                let conn_text = if s.connected { "● CONNECTED" } else { "● DISCONNECTED" };
                ui.colored_label(conn_color, conn_text);
                ui.separator();

                ui.label(format!("Mode: {}", s.mode));
                ui.separator();

                let arm_color = if s.armed { egui::Color32::RED } else { egui::Color32::GREEN };
                let arm_text = if s.armed { "ARMED" } else { "DISARMED" };
                ui.colored_label(arm_color, arm_text);
                ui.separator();

                ui.label(format!("Bat: {:.1}V {}%", s.voltage, s.battery_remaining));
            });
        });

        // Bottom panel — command input
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(">");
                let response = ui.text_edit_singleline(&mut self.input);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let cmd = self.input.trim().to_string();
                    if !cmd.is_empty() {
                        self.send_command(&cmd);
                        self.input.clear();
                    }
                    response.request_focus();
                }
                if ui.button("Send").clicked() {
                    let cmd = self.input.trim().to_string();
                    if !cmd.is_empty() {
                        self.send_command(&cmd);
                        self.input.clear();
                    }
                }

                // Quick buttons
                ui.separator();
                if ui.button("ARM").clicked() { self.send_command("arm"); }
                if ui.button("DISARM").clicked() { self.send_command("disarm"); }
                if ui.button("TAKEOFF").clicked() { self.send_command("takeoff 10"); }
                if ui.button("RTL").clicked() { self.send_command("rtl"); }
                if ui.button("LAND").clicked() { self.send_command("land"); }
            });
        });

        // Right panel — telemetry
        egui::SidePanel::right("telemetry").min_width(220.0).show(ctx, |ui| {
            ui.heading("GPS");
            egui::Grid::new("gps_grid").show(ui, |ui| {
                ui.label("Lat:");
                ui.label(format!("{:.7}", s.lat));
                ui.end_row();
                ui.label("Lon:");
                ui.label(format!("{:.7}", s.lon));
                ui.end_row();
                ui.label("Alt:");
                ui.label(format!("{:.1}m", s.relative_alt));
                ui.end_row();
                ui.label("Hdg:");
                ui.label(format!("{}°", s.heading));
                ui.end_row();
                ui.label("Sat:");
                ui.label(format!("{} ({})", s.satellites, crate::vehicle::fix_type_name(s.fix_type)));
                ui.end_row();
            });

            ui.separator();
            ui.heading("Attitude");
            egui::Grid::new("att_grid").show(ui, |ui| {
                ui.label("Roll:");
                ui.label(format!("{:.2}°", s.roll));
                ui.end_row();
                ui.label("Pitch:");
                ui.label(format!("{:.2}°", s.pitch));
                ui.end_row();
                ui.label("Yaw:");
                ui.label(format!("{:.2}°", s.yaw));
                ui.end_row();
            });

            ui.separator();
            ui.heading("Battery");
            let pct = if s.battery_remaining >= 0 { s.battery_remaining as f32 / 100.0 } else { 0.0 };
            let bar_color = match (pct * 100.0) as u8 {
                0..=20 => egui::Color32::RED,
                21..=50 => egui::Color32::YELLOW,
                _ => egui::Color32::GREEN,
            };
            ui.horizontal(|ui| {
                ui.label(format!("{:.1}V", s.voltage));
                let bar = egui::ProgressBar::new(pct)
                    .text(format!("{}%", s.battery_remaining))
                    .fill(bar_color);
                ui.add(bar);
            });

            ui.separator();
            ui.heading("Log");
            egui::ScrollArea::vertical().max_height(200.0).stick_to_bottom(true).show(ui, |ui| {
                for entry in s.log.iter() {
                    ui.label(egui::RichText::new(entry).monospace().size(11.0));
                }
            });
        });

        // Central panel — video
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.video_texture {
                let available = ui.available_size();
                let tex_size = tex.size_vec2();
                let scale = (available.x / tex_size.x).min(available.y / tex_size.y);
                let size = egui::vec2(tex_size.x * scale, tex_size.y * scale);
                ui.centered_and_justified(|ui| {
                    ui.image(egui::load::SizedTexture::new(tex.id(), size));
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new("No Video Feed\n\nWaiting for GStreamer stream on UDP:5600\nor use --test-video for test pattern")
                        .size(18.0)
                        .color(egui::Color32::GRAY));
                });
            }
        });

        // Request repaint for live updates
        ctx.request_repaint();
    }
}
