use chrono::NaiveDate;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::{Buffer, StatefulWidget},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{
        Block, Borders, Cell, Padding, Paragraph, Row, Scrollbar, ScrollbarState, Table, Widget,
        Wrap,
    },
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, io};
use tokio;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Jotform {
    id: String,
    submitter_name: FullName,
    created_at: SubmissionDate,
    location: String,
    exhibit_name: String,
    description: String,
    priority_level: String,
    department: String,
    status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FullName {
    first: String,
    last: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SubmissionDate {
    date: String,
    time: String,
}

#[derive(Debug, Default)]
struct App {
    jotforms: Vec<Jotform>,
    selected_id: String,
    scroll_state: ScrollbarState,
    description_offset: u16,
    exit: bool,
}

impl App {
    async fn setup_initial_state(&mut self) -> Result<(), Box<dyn Error>> {
        self.jotforms = fetch_jotforms().await?;
        if let Some(first_jotform) = self.jotforms.first() {
            self.selected_id = first_jotform.id.clone();
        }
        Ok(())
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Handle the Result from setup_initial_state
        if let Err(e) = self.setup_initial_state().await {
            eprintln!("Failed to setup initial state: {}", e);
            return Ok(()); // or return Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        }

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            // Handle the Result from handle_events
            if let Err(e) = self.handle_events().await {
                eprintln!("Error handling events: {}", e);
                break;
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    async fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_events(key_event).await;
            }
            _ => {}
        };
        Ok(())
    }

    async fn handle_key_events(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),

            KeyCode::Up => {
                if let Some(current_index) =
                    self.jotforms.iter().position(|j| j.id == self.selected_id)
                {
                    if current_index > 0 {
                        self.selected_id = self.jotforms[current_index - 1].id.clone();
                        self.description_offset = 0;
                    }
                }
            }
            KeyCode::Down => {
                if let Some(current_index) =
                    self.jotforms.iter().position(|j| j.id == self.selected_id)
                {
                    if current_index < self.jotforms.len() - 1 {
                        self.selected_id = self.jotforms[current_index + 1].id.clone();
                        self.description_offset = 0;
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(selected_jotform) =
                    self.jotforms.iter_mut().find(|j| j.id == self.selected_id)
                {
                    selected_jotform.status = match selected_jotform.status.as_str() {
                        "Open" => "InProgress".to_string(),
                        "InProgress" => "Closed".to_string(),
                        "Closed" => "Unplanned".to_string(),
                        "Unplanned" => "Open".to_string(),
                        _ => "Open".to_string(),
                    };
                    update_status(&selected_jotform.id, &selected_jotform.status)
                        .await
                        .unwrap();
                    self.jotforms.sort_by(|a, b| {
                        let status_order = match (a.status.as_str(), b.status.as_str()) {
                            ("InProgress", _) => std::cmp::Ordering::Less,
                            (_, "InProgress") => std::cmp::Ordering::Greater,
                            ("Unplanned", _) => std::cmp::Ordering::Greater,
                            (_, "Unplanned") => std::cmp::Ordering::Less,
                            _ => std::cmp::Ordering::Equal,
                        };
                        if status_order == std::cmp::Ordering::Equal {
                            let date_a =
                                NaiveDate::parse_from_str(&a.created_at.date, "%Y-%m-%d").unwrap();
                            let date_b =
                                NaiveDate::parse_from_str(&b.created_at.date, "%Y-%m-%d").unwrap();
                            date_b.cmp(&date_a)
                        } else {
                            status_order
                        }
                    });
                    if let Some(new_index) =
                        self.jotforms.iter().position(|j| j.id == self.selected_id)
                    {
                        self.selected_id = self.jotforms[new_index].id.clone();
                    }
                }
            }

            KeyCode::PageUp => {
                self.description_offset = self.description_offset.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.description_offset = self.description_offset.saturating_add(1);
            }

            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        // Render the table
        let rows = self.jotforms.iter().map(|jotform| {
            let is_selected = jotform.id == self.selected_id;
            let formatted_date = NaiveDate::parse_from_str(&jotform.created_at.date, "%Y-%m-%d")
                .map(|date| date.format("%m-%d-%Y").to_string())
                .unwrap_or_else(|_| jotform.created_at.date.clone());

            let status_style = match jotform.status.as_str() {
                "Open" => Style::default().fg(Color::Rgb(144, 238, 144)),
                "Closed" => Style::default().fg(Color::Rgb(255, 182, 193)),
                "InProgress" => Style::default().fg(Color::Rgb(216, 191, 216)),
                "Unplanned" => Style::default().fg(Color::Rgb(105, 105, 105)),
                _ => Style::default().fg(Color::DarkGray),
            };
            let priority_style = match jotform.priority_level.as_str() {
                "Low" => Style::default().fg(Color::Rgb(144, 238, 144)),
                "Medium" => Style::default().fg(Color::Rgb(255, 255, 153)),
                "High" => Style::default().fg(Color::Rgb(255, 182, 193)),
                _ => Style::default().fg(Color::DarkGray),
            };
            let department_style = match jotform.department.as_str() {
                "Exhibits" => Style::default().fg(Color::Rgb(255, 183, 82)),
                "Operations" => Style::default().fg(Color::Rgb(173, 216, 230)),
                _ => Style::default().fg(Color::DarkGray),
            };
            let row_style = if is_selected {
                Style::default().bg(Color::Rgb(70, 70, 90))
            } else {
                Style::default().bg(Color::Rgb(30, 30, 40))
            };

            Row::new(vec![
                Cell::from(jotform.submitter_name.first.clone()),
                Cell::from(formatted_date),
                Cell::from(jotform.location.clone()),
                Cell::from(jotform.exhibit_name.clone()),
                Cell::from(Span::styled(jotform.priority_level.clone(), priority_style)),
                Cell::from(Span::styled(jotform.department.clone(), department_style)),
                Cell::from(Span::styled(jotform.status.clone(), status_style)),
            ])
            .style(row_style)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
                Constraint::Percentage(14),
            ],
        )
        .header(
            Row::new(vec![
                "Submitter",
                "Date",
                "Location",
                "Exhibit",
                "Priority",
                "Department",
                "Status",
            ])
            .style(
                Style::default()
                    .fg(Color::Rgb(200, 200, 200))
                    .bg(Color::Rgb(50, 50, 60))
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(100, 100, 120)))
                .title("Jotforms")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(150, 150, 170))
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .footer(
            Row::new(vec![
                "↑/↓: Navigate Jotforms",
                "E: Change Status",
                "Q: Quit",
            ])
            .style(
                Style::default()
                    .fg(Color::Rgb(200, 200, 200))
                    .bg(Color::Rgb(50, 50, 60))
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .column_spacing(2);

        Widget::render(table, chunks[0], buf);

        let selected_jotform = self.jotforms.iter().find(|j| j.id == self.selected_id);
        let description = match selected_jotform {
            Some(j) => j.description.clone(),
            None => "Select a Jotform to view description".to_string(),
        };

        let description_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(100, 100, 120)))
            .title("Description")
            .title_style(
                Style::default()
                    .fg(Color::Rgb(150, 150, 170))
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::new(1, 1, 1, 1))
            .style(
                Style::default()
                    .bg(Color::Rgb(30, 30, 40))
                    .fg(Color::Rgb(200, 200, 200)),
            );

        let desc_paragraph = Paragraph::new(description.clone())
            .block(description_block)
            .wrap(Wrap { trim: false })
            .scroll((self.description_offset, 0));

        desc_paragraph.render(chunks[1], buf);

        let total_lines = description.lines().count();
        let visible_lines = chunks[1].height.saturating_sub(2) as usize;

        let scroll_state = self
            .scroll_state
            .content_length(total_lines)
            .viewport_content_length(visible_lines)
            .position(self.description_offset as usize);

        let scrollbar = Scrollbar::default()
            .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        scrollbar.render(chunks[1], buf, &mut scroll_state.clone());
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal).await;
    ratatui::restore();
    app_result
}

async fn fetch_jotforms() -> Result<Vec<Jotform>, Box<dyn Error>> {
    let response = reqwest::get("http://localhost:3030/jotforms").await?;
    let jotforms = response.json::<Vec<Jotform>>().await?;
    Ok(jotforms)
}

async fn update_status(id: &str, status: &str) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://localhost:3030/jotforms/{}/status", id))
        .json(&serde_json::json!({ "new_status": status }))
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("Failed to update status: {}", response.status());
    }
    Ok(())
}
