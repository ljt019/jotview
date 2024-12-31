use chrono::NaiveDate;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{
        Block, Borders, Cell, Padding, Paragraph, Row, Scrollbar, ScrollbarState, Table, Wrap,
    },
    Terminal,
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
        let status_order = match (a.status.as_str(), b.status.as_str()) {
            ("InProgress", _) => std::cmp::Ordering::Less,
            (_, "InProgress") => std::cmp::Ordering::Greater,
            ("Unplanned", _) => std::cmp::Ordering::Greater,
            (_, "Unplanned") => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        };
        if status_order == std::cmp::Ordering::Equal {
            let date_a = NaiveDate::parse_from_str(&a.created_at.date, "%Y-%m-%d").unwrap();
            let date_b = NaiveDate::parse_from_str(&b.created_at.date, "%Y-%m-%d").unwrap();
            date_b.cmp(&date_a)
        } else {
            status_order
        }
    });

    let mut selected_id = jotforms.get(0).map(|j| j.id.clone()).unwrap_or_default();

    // The ratatui ScrollbarState
    let mut scroll_state = ScrollbarState::default();

    // We'll keep a separate offset for the paragraph scrolling:
    let mut description_offset: u16 = 0;

    loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(size);

            // Render the table
            let rows = jotforms.iter().map(|jotform| {
                let is_selected = jotform.id == selected_id;
                let formatted_date =
                    NaiveDate::parse_from_str(&jotform.created_at.date, "%Y-%m-%d")
                        .map(|date| date.format("%m-%d-%Y").to_string())
                        .unwrap_or_else(|_| jotform.created_at.date.clone());

                let status_style = match jotform.status.as_str() {
                    "Open" => Style::default().fg(Color::Rgb(144, 238, 144)), // Pastel green
                    "Closed" => Style::default().fg(Color::Rgb(255, 182, 193)), // Pastel red
                    "InProgress" => Style::default().fg(Color::Rgb(216, 191, 216)), // Pastel purple
                    "Unplanned" => Style::default().fg(Color::Rgb(105, 105, 105)), // Dark dim grey
                    _ => Style::default().fg(Color::DarkGray),
                };
                let priority_style = match jotform.priority_level.as_str() {
                    "Low" => Style::default().fg(Color::Rgb(144, 238, 144)), // Pastel green
                    "Medium" => Style::default().fg(Color::Rgb(255, 255, 153)), // Pastel yellow
                    "High" => Style::default().fg(Color::Rgb(255, 182, 193)), // Pastel red
                    _ => Style::default().fg(Color::DarkGray),
                };
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
                        .fg(Color::Rgb(200, 200, 200)) // Light pastel gray for header text
                        .bg(Color::Rgb(50, 50, 60)) // Dark pastel background for header
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 120))) // Dark pastel border
                    .title("Jotforms")
                    .title_style(
                        Style::default()
                            .fg(Color::Rgb(150, 150, 170)) // Light pastel title color
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
                        .fg(Color::Rgb(200, 200, 200)) // Light pastel gray for footer text
                        .bg(Color::Rgb(50, 50, 60)) // Dark pastel background for footer
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .column_spacing(2);

            f.render_widget(table, chunks[0]);

            //
            // === DESCRIPTION + SCROLLBAR ===
            //

            // Figure out which description to show
            let selected_jotform = jotforms.iter().find(|j| j.id == selected_id);
            let description = match selected_jotform {
                Some(j) => j.description.clone(),
                None => "Select a Jotform to view description".to_string(),
            };

            // Create a block for our paragraph
            let description_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(100, 100, 120))) // Dark pastel border
                .title("Description")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(150, 150, 170)) // Light pastel title color
                        .add_modifier(Modifier::BOLD),
                )
                .padding(Padding::new(1, 1, 1, 1)) // Add padding so text isn't flush to the edge
                .style(
                    Style::default()
                        .bg(Color::Rgb(30, 30, 40)) // Dark pastel background
                        .fg(Color::Rgb(200, 200, 200)), // Light pastel text
                );

            // Build the Paragraph. Notice we pass (description_offset, 0) to scroll vertically
            let desc_paragraph = Paragraph::new(description.clone())
                .block(description_block)
                .wrap(Wrap { trim: false })
                .scroll((description_offset, 0));

            // Render the paragraph (which includes the block)
            f.render_widget(desc_paragraph, chunks[1]);

            // We also tell the scrollbar how big the text is vs. how many lines fit:
            //
            // For a simplistic approach, let's just treat the number of lines as
            // the number of `\n`-separated lines. A more robust approach might
            // measure wrapped lines.
            let total_lines = description.lines().count();
            // How many lines can fit in the chunk? We subtract top/bottom padding
            // if you want to be precise. For simplicity, just use the chunk height:
            let visible_lines = chunks[1].height.saturating_sub(2) as usize; // minus borders, etc.

            // Update the scrollbar's state
            // content_length -> total lines in the text
            // viewport_content_length -> how many lines can be displayed
            // position -> your current offset
            scroll_state = scroll_state
                .content_length(total_lines)
                .viewport_content_length(visible_lines)
                .position(description_offset as usize);

            // Build and render the scrollbar
            let scrollbar = Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            f.render_stateful_widget(scrollbar, chunks[1], &mut scroll_state);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,

                KeyCode::Up => {
                    // Move selection up in the table
                    if let Some(current_index) = jotforms.iter().position(|j| j.id == selected_id) {
                        if current_index > 0 {
                            selected_id = jotforms[current_index - 1].id.clone();
                            // Reset the paragraph offset & scrollbar
                            description_offset = 0;
                        }
                    }
                }
                KeyCode::Down => {
                    // Move selection down in the table
                    if let Some(current_index) = jotforms.iter().position(|j| j.id == selected_id) {
                        if current_index < jotforms.len() - 1 {
                            selected_id = jotforms[current_index + 1].id.clone();
                            // Reset the paragraph offset & scrollbar
                            description_offset = 0;
                        }
                    }
                }
                KeyCode::Char('e') => {
                    // Cycle status
                    if let Some(selected_jotform) =
                        jotforms.iter_mut().find(|j| j.id == selected_id)
                    {
                        selected_jotform.status = match selected_jotform.status.as_str() {
                            "Open" => "InProgress".to_string(),
                            "InProgress" => "Closed".to_string(),
                            "Closed" => "Unplanned".to_string(),
                            "Unplanned" => "Open".to_string(),
                            _ => "Open".to_string(),
                        };
                        update_status(&selected_jotform.id, &selected_jotform.status).await?;
                        jotforms.sort_by(|a, b| {
                            let status_order = match (a.status.as_str(), b.status.as_str()) {
                                ("InProgress", _) => std::cmp::Ordering::Less,
                                (_, "InProgress") => std::cmp::Ordering::Greater,
                                ("Unplanned", _) => std::cmp::Ordering::Greater,
                                (_, "Unplanned") => std::cmp::Ordering::Less,
                                _ => std::cmp::Ordering::Equal,
                            };
                            if status_order == std::cmp::Ordering::Equal {
                                let date_a =
                                    NaiveDate::parse_from_str(&a.created_at.date, "%Y-%m-%d")
                                        .unwrap();
                                let date_b =
                                    NaiveDate::parse_from_str(&b.created_at.date, "%Y-%m-%d")
                                        .unwrap();
                                date_b.cmp(&date_a)
                            } else {
                                status_order
                            }
                        });
                        if let Some(new_index) = jotforms.iter().position(|j| j.id == selected_id) {
                            selected_id = jotforms[new_index].id.clone();
                        }
                    }
                }

                // PageUp: scroll the description up
                KeyCode::PageUp => {
                    description_offset = description_offset.saturating_sub(1);
                }
                // PageDown: scroll down
                KeyCode::PageDown => {
                    // In a real app, you’d want to clamp at total_lines - visible_lines
                    description_offset = description_offset.saturating_add(1);
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
        .post(format!("http://localhost:3030/jotforms/{}/status", id))
        .json(&serde_json::json!({ "new_status": status }))
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("Failed to update status: {}", response.status());
    }
    Ok(())
}
