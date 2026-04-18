use std::sync::{Arc, Mutex};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::vehicle::{VehicleState, fix_type_name};

pub struct App {
    pub input: String,
    pub should_quit: bool,
    pub pending_commands: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            should_quit: false,
            pending_commands: Vec::new(),
        }
    }

    pub fn handle_key_event(&mut self) -> std::io::Result<()> {
        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Esc => self.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                KeyCode::Enter => {
                    let trimmed = self.input.trim();
                    if !trimmed.is_empty() {
                        if trimmed == "q" || trimmed == "quit" {
                            self.should_quit = true;
                        } else {
                            self.pending_commands.push(self.input.clone());
                        }
                        self.input.clear();
                    }
                }
                KeyCode::Char(c) => self.input.push(c),
                KeyCode::Backspace => { self.input.pop(); }
                _ => {}
            }
        }
        Ok(())
    }
}

pub fn draw(frame: &mut Frame, state: &Arc<Mutex<VehicleState>>, app: &App) {
    let s = state.lock().unwrap().clone();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(main_chunks[0]);

    // Status
    let conn_status = if s.connected { "✓ CONNECTED" } else { "✗ DISCONNECTED" };
    let conn_color = if s.connected { Color::Green } else { Color::Red };
    let armed_str = if s.armed { "ARMED" } else { "DISARMED" };
    let armed_color = if s.armed { Color::Red } else { Color::Green };

    let status = Paragraph::new(vec![
        Line::from(vec![Span::raw("Conn: "), Span::styled(conn_status, Style::default().fg(conn_color))]),
        Line::from(vec![Span::raw("Mode: "), Span::styled(&s.mode, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::raw("Arm:  "), Span::styled(armed_str, Style::default().fg(armed_color))]),
        Line::from(format!("Stat: {}", s.system_status)),
    ]).block(Block::default().title(" Status ").borders(Borders::ALL));
    frame.render_widget(status, top_chunks[0]);

    // GPS
    let gps = Paragraph::new(vec![
        Line::from(format!("Lat: {:.7}", s.lat)),
        Line::from(format!("Lon: {:.7}", s.lon)),
        Line::from(format!("Alt: {:.1}m (rel: {:.1}m)", s.alt, s.relative_alt)),
        Line::from(format!("Hdg: {}°  Sat: {} ({})", s.heading, s.satellites, fix_type_name(s.fix_type))),
    ]).block(Block::default().title(" GPS ").borders(Borders::ALL));
    frame.render_widget(gps, top_chunks[1]);

    // Attitude
    let attitude = Paragraph::new(vec![
        Line::from(format!("Roll:  {:>7.2}°", s.roll)),
        Line::from(format!("Pitch: {:>7.2}°", s.pitch)),
        Line::from(format!("Yaw:   {:>7.2}°", s.yaw)),
    ]).block(Block::default().title(" Attitude ").borders(Borders::ALL));
    frame.render_widget(attitude, top_chunks[2]);

    // Battery
    let batt_pct = if s.battery_remaining >= 0 { s.battery_remaining as u16 } else { 0 };
    let batt_color = match batt_pct {
        0..=20 => Color::Red,
        21..=50 => Color::Yellow,
        _ => Color::Green,
    };
    let battery = Gauge::default()
        .block(Block::default().title(" Battery ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(batt_color))
        .percent(batt_pct)
        .label(format!("{:.1}V  {}%", s.voltage, s.battery_remaining));
    frame.render_widget(battery, top_chunks[3]);

    // Log
    let skip = s.log.len().saturating_sub(20);
    let log_items: Vec<ListItem> = s.log.iter().skip(skip)
        .map(|entry| ListItem::new(Line::from(entry.as_str())))
        .collect();
    let log_list = List::new(log_items)
        .block(Block::default().title(" Log ").borders(Borders::ALL));
    frame.render_widget(log_list, main_chunks[1]);

    // Input
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(&app.input),
        Span::styled("_", Style::default().fg(Color::Gray)),
    ])).block(Block::default().title(" Command (q=quit) ").borders(Borders::ALL));
    frame.render_widget(input, main_chunks[2]);
}
