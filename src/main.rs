use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Row, Table, Cell},
    Terminal,
};
use ratatui::text::Span;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::{Deserialize, Serialize};
use std::{error::Error, io};
use tokio;
use chrono::NaiveDate;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Fetch Jotforms data
    let mut jotforms = fetch_jotforms().await?;

    // Sort the jotforms by status and date (descending)
    jotforms.sort_by(|a, b| {
        // First, sort by status
        let status_order = match (a.status.as_str(), b.status.as_str()) {
            ("InProgress", _) => std::cmp::Ordering::Less,
            (_, "InProgress") => std::cmp::Ordering::Greater,
            ("Unplanned", _) => std::cmp::Ordering::Greater,
            (_, "Unplanned") => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal, // "Open" and "Closed" are treated as equal
        };

        // If status is the same, sort by date in descending order
        if status_order == std::cmp::Ordering::Equal {
            // Parse the dates for comparison
            let date_a = NaiveDate::parse_from_str(&a.created_at.date, "%Y-%m-%d").unwrap();
            let date_b = NaiveDate::parse_from_str(&b.created_at.date, "%Y-%m-%d").unwrap();
            date_b.cmp(&date_a) // Reverse the order for descending
        } else {
            status_order
        }
    });

    let mut selected_id = jotforms.get(0).map(|j| j.id.clone()).unwrap_or_default();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(size);

            let rows = jotforms.iter().map(|jotform| {
                let is_selected = jotform.id == selected_id;

                // Parse and reformat the date
                let formatted_date = NaiveDate::parse_from_str(&jotform.created_at.date, "%Y-%m-%d")
                    .map(|date| date.format("%m-%d-%Y").to_string())
                    .unwrap_or_else(|_| jotform.created_at.date.clone());

                // Status color coding
                let status_style = match jotform.status.as_str() {
                    "Open" => Style::default().fg(Color::Rgb(144, 238, 144)), // Pastel green
                    "Closed" => Style::default().fg(Color::Rgb(255, 182, 193)), // Pastel red
                    "InProgress" => Style::default().fg(Color::Rgb(216, 191, 216)), // Pastel purple
                    "Unplanned" => Style::default().fg(Color::Rgb(105, 105, 105)), // Dark dim grey
                    _ => Style::default().fg(Color::DarkGray),
                };

                // Priority color coding
                let priority_style = match jotform.priority_level.as_str() {
                    "Low" => Style::default().fg(Color::Rgb(144, 238, 144)), // Pastel green
                    "Medium" => Style::default().fg(Color::Rgb(255, 255, 153)), // Pastel yellow
                    "High" => Style::default().fg(Color::Rgb(255, 182, 193)), // Pastel red
                    _ => Style::default().fg(Color::DarkGray),
                };

                // Department color coding
                let department_style = match jotform.department.as_str() {
                    "Exhibits" => Style::default().fg(Color::Rgb(255, 183, 82)), // Pastel orange
                    "Operations" => Style::default().fg(Color::Rgb(173, 216, 230)), // Pastel blue
                    _ => Style::default().fg(Color::DarkGray),
                };

                let row_style = if is_selected {
                    Style::default().bg(Color::Rgb(70, 70, 90)) // Dark pastel blue for selected row
                } else {
                    Style::default().bg(Color::Rgb(30, 30, 40)) // Dark pastel background for rows
                };

                Row::new(vec![
                    Cell::from(jotform.submitter_name.first.clone()),
                    Cell::from(formatted_date), // Use the formatted date
                    Cell::from(jotform.location.clone()),
                    Cell::from(jotform.exhibit_name.clone()),
                    Cell::from(Span::styled(jotform.priority_level.clone(), priority_style)), // Color-code priority
                    Cell::from(Span::styled(jotform.department.clone(), department_style)), // Color-code department
                    Cell::from(Span::styled(jotform.status.clone(), status_style)), // Color-code status
                ])
                .style(row_style)
            });

            // Adjust the constraints to match the new columns
            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(14), // First Name
                    Constraint::Percentage(14), // Date
                    Constraint::Percentage(14), // Location
                    Constraint::Percentage(14), // Exhibit Name
                    Constraint::Percentage(14), // Priority Level
                    Constraint::Percentage(14), // Department
                    Constraint::Percentage(14), // Status
                ]
            )
            .header(
                Row::new(vec![
                    "Submitter", "Date", "Location", "Exhibit", "Priority", "Department", "Status",
                ])
                .style(Style::default()
                    .fg(Color::Rgb(200, 200, 200)) // Light pastel gray for header text
                    .bg(Color::Rgb(50, 50, 60)) // Dark pastel background for header
                    .add_modifier(Modifier::BOLD) // Bold header text
                )
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 120))) // Dark pastel border
                    .title("Jotforms")
                    .title_style(Style::default()
                        .fg(Color::Rgb(150, 150, 170)) // Light pastel title color
                        .add_modifier(Modifier::BOLD) // Bold title
                    )
            )
            .footer(
                Row::new(vec![
                    "↑/↓: Navigate", "E: Change Status", "Q: Quit",
                ])
                .style(Style::default()
                    .fg(Color::Rgb(200, 200, 200)) // Light pastel gray for footer text
                    .bg(Color::Rgb(50, 50, 60)) // Dark pastel background for footer
                    .add_modifier(Modifier::BOLD) // Bold footer text
                )
            )
            .column_spacing(2); // Add spacing between columns

            f.render_widget(table, chunks[0]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break, // Quit
                KeyCode::Up => {
                    if let Some(current_index) = jotforms.iter().position(|j| j.id == selected_id) {
                        if current_index > 0 {
                            selected_id = jotforms[current_index - 1].id.clone();
                        }
                    }
                }
                KeyCode::Down => {
                    if let Some(current_index) = jotforms.iter().position(|j| j.id == selected_id) {
                        if current_index < jotforms.len() - 1 {
                            selected_id = jotforms[current_index + 1].id.clone();
                        }
                    }
                }
                KeyCode::Char('e') => {
                    // Change status for the selected row
                    if let Some(selected_jotform) = jotforms.iter_mut().find(|j| j.id == selected_id) {
                        selected_jotform.status = match selected_jotform.status.as_str() {
                            "Open" => "InProgress".to_string(),
                            "InProgress" => "Closed".to_string(),
                            "Closed" => "Unplanned".to_string(),
                            "Unplanned" => "Open".to_string(),
                            _ => "Open".to_string(),
                        };
                        update_status(&selected_jotform.id, &selected_jotform.status).await?;

                        // Re-sort the jotforms after changing the status
                        jotforms.sort_by(|a, b| {
                            // First, sort by status
                            let status_order = match (a.status.as_str(), b.status.as_str()) {
                                ("InProgress", _) => std::cmp::Ordering::Less,
                                (_, "InProgress") => std::cmp::Ordering::Greater,
                                ("Unplanned", _) => std::cmp::Ordering::Greater,
                                (_, "Unplanned") => std::cmp::Ordering::Less,
                                _ => std::cmp::Ordering::Equal, // "Open" and "Closed" are treated as equal
                            };

                            // If status is the same, sort by date in descending order
                            if status_order == std::cmp::Ordering::Equal {
                                // Parse the dates for comparison
                                let date_a = NaiveDate::parse_from_str(&a.created_at.date, "%Y-%m-%d").unwrap();
                                let date_b = NaiveDate::parse_from_str(&b.created_at.date, "%Y-%m-%d").unwrap();
                                date_b.cmp(&date_a) // Reverse the order for descending
                            } else {
                                status_order
                            }
                        });

                        // Find the new index of the selected row after sorting
                        if let Some(new_index) = jotforms.iter().position(|j| j.id == selected_id) {
                            selected_id = jotforms[new_index].id.clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    restore_terminal();
    Ok(())
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

async fn fetch_jotforms() -> Result<Vec<Jotform>, Box<dyn Error>> {
    let response = reqwest::get("http://localhost:3030/jotforms").await?;
    let jotforms = response.json::<Vec<Jotform>>().await?;
    Ok(jotforms)
}

async fn update_status(id: &str, status: &str) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://localhost:3030/jotforms/{}/status", id)) // Correct endpoint URL
        .json(&serde_json::json!({ "new_status": status })) // Correct JSON payload
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("Failed to update status: {}", response.status());
    }

    Ok(())
}