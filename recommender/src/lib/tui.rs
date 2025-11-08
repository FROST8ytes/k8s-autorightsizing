use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use std::collections::HashSet;
use std::io;
use url::Url;

use crate::lib::output::RecommenderOutput;

/// Application mode
#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    BrowsingTable,
    ConfirmApply,
    InputUrl,
    InputToken,
    InputBranch,
    Applying,
    ShowResult(String, Option<String>), // (message, pr_url)
}

/// Application state
struct AppState {
    table_state: TableState,
    selected_indices: HashSet<usize>,
    mode: AppMode,
    input_buffer: String,
    error_message: Option<String>,
}

impl AppState {
    fn new(total_items: usize) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            table_state,
            selected_indices: (0..total_items).collect(), // Select all by default
            mode: AppMode::BrowsingTable,
            input_buffer: String::new(),
            error_message: None,
        }
    }
}

/// Display recommendations in an interactive table
pub fn display_recommendations_table(output: RecommenderOutput) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let res = run_recommendations_app(&mut terminal, output, None, String::from("main"), None);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_recommendations_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    output: RecommenderOutput,
    manifest_url: Option<Url>,
    git_branch: String,
    git_token: Option<String>,
) -> io::Result<()> {
    let total_items = output.recommendations.len();
    let mut state = AppState::new(total_items);

    loop {
        terminal.draw(|f| {
            let area = f.area();

            // Extract mode to avoid borrow conflicts
            let mode = state.mode.clone();
            let selected_count = state.selected_indices.len();

            match mode {
                AppMode::BrowsingTable => {
                    render_table(f, area, &output, &state);
                }
                AppMode::ConfirmApply => {
                    render_table(f, area, &output, &state);
                    render_confirm_dialog(f, area, selected_count);
                }
                AppMode::InputUrl => {
                    render_table(f, area, &output, &state);
                    render_input_dialog(
                        f,
                        area,
                        "Enter Git Repository URL",
                        &state.input_buffer,
                        state.error_message.as_deref(),
                    );
                }
                AppMode::InputToken => {
                    render_table(f, area, &output, &state);
                    let masked = "*".repeat(state.input_buffer.len());
                    render_input_dialog(
                        f,
                        area,
                        "Enter Git Token (optional)",
                        &masked,
                        state.error_message.as_deref(),
                    );
                }
                AppMode::InputBranch => {
                    render_table(f, area, &output, &state);
                    render_input_dialog(
                        f,
                        area,
                        "Enter Branch Name",
                        &state.input_buffer,
                        state.error_message.as_deref(),
                    );
                }
                AppMode::Applying => {
                    render_table(f, area, &output, &state);
                    render_progress_dialog(f, area);
                }
                AppMode::ShowResult(ref message, ref pr_url) => {
                    render_table(f, area, &output, &state);
                    render_result_dialog(f, area, message, pr_url.as_deref());
                }
            }
        })?;

        // Handle input based on mode
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match &state.mode {
                    AppMode::BrowsingTable => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char(' ') => {
                                if let Some(i) = state.table_state.selected() {
                                    if state.selected_indices.contains(&i) {
                                        state.selected_indices.remove(&i);
                                    } else {
                                        state.selected_indices.insert(i);
                                    }
                                }
                            }
                            KeyCode::Char('a') => {
                                // Select all
                                state.selected_indices = (0..total_items).collect();
                            }
                            KeyCode::Char('n') => {
                                // Deselect all
                                state.selected_indices.clear();
                            }
                            KeyCode::Enter => {
                                if !state.selected_indices.is_empty() {
                                    state.mode = AppMode::ConfirmApply;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let i = match state.table_state.selected() {
                                    Some(i) => {
                                        if i >= total_items - 1 {
                                            0
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                state.table_state.select(Some(i));
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                let i = match state.table_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            total_items - 1
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                state.table_state.select(Some(i));
                            }
                            _ => {}
                        }
                    }
                    AppMode::ConfirmApply => {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                // Proceed to URL input or apply directly
                                if manifest_url.is_none() {
                                    state.mode = AppMode::InputUrl;
                                    state.input_buffer.clear();
                                    state.error_message = None;
                                } else if git_token.is_none() {
                                    state.mode = AppMode::InputToken;
                                    state.input_buffer.clear();
                                    state.error_message = None;
                                } else {
                                    // Apply directly (async operation would go here)
                                    state.mode = AppMode::ShowResult(
                                        "Feature requires async support in TUI".to_string(),
                                        None,
                                    );
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                state.mode = AppMode::BrowsingTable;
                            }
                            _ => {}
                        }
                    }
                    AppMode::InputUrl | AppMode::InputToken | AppMode::InputBranch => {
                        match key.code {
                            KeyCode::Enter => {
                                handle_input_submit(
                                    &mut state,
                                    &manifest_url,
                                    &git_token,
                                    &git_branch,
                                );
                            }
                            KeyCode::Esc => {
                                state.mode = AppMode::BrowsingTable;
                                state.input_buffer.clear();
                                state.error_message = None;
                            }
                            KeyCode::Char(c) => {
                                state.input_buffer.push(c);
                                state.error_message = None;
                            }
                            KeyCode::Backspace => {
                                state.input_buffer.pop();
                                state.error_message = None;
                            }
                            _ => {}
                        }
                    }
                    AppMode::ShowResult(_, _) => {
                        // Any key returns to browsing
                        return Ok(());
                    }
                    AppMode::Applying => {
                        // No input during applying
                    }
                }
            }
        }
    }
}

fn handle_input_submit(
    state: &mut AppState,
    _manifest_url: &Option<Url>,
    _git_token: &Option<String>,
    git_branch: &str,
) {
    match &state.mode {
        AppMode::InputUrl => {
            // Validate URL
            match Url::parse(&state.input_buffer) {
                Ok(_) => {
                    state.mode = AppMode::InputToken;
                    state.input_buffer.clear();
                    state.error_message = None;
                }
                Err(e) => {
                    state.error_message = Some(format!("Invalid URL: {}", e));
                }
            }
        }
        AppMode::InputToken => {
            // Token is optional, move to branch input
            state.mode = AppMode::InputBranch;
            state.input_buffer = git_branch.to_string();
            state.error_message = None;
        }
        AppMode::InputBranch => {
            // Would trigger actual apply here
            state.mode = AppMode::ShowResult(
                "Apply workflow requires async support. Use CLI mode (--apply) instead."
                    .to_string(),
                None,
            );
        }
        _ => {}
    }
}

fn render_table(f: &mut ratatui::Frame, area: Rect, output: &RecommenderOutput, state: &AppState) {
    // Create the table header
    let header_cells = [
        "✓",
        "Namespace",
        "Deployment",
        "Container",
        "CPU Req (Current → Rec)",
        "CPU Lim (Current → Rec)",
        "Mem Req (Current → Rec)",
        "Mem Lim (Current → Rec)",
    ]
    .iter()
    .map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    // Create table rows with selection indicators
    let rows = output.recommendations.iter().enumerate().map(|(idx, rec)| {
        let selected_mark = if state.selected_indices.contains(&idx) {
            "✓"
        } else {
            " "
        };

        let cpu_req_change =
            get_change_indicator(&rec.current_cpu_request, &rec.recommended_cpu_request);
        let cpu_lim_change =
            get_change_indicator(&rec.current_cpu_limit, &rec.recommended_cpu_limit);
        let mem_req_change =
            get_change_indicator(&rec.current_memory_request, &rec.recommended_memory_request);
        let mem_lim_change =
            get_change_indicator(&rec.current_memory_limit, &rec.recommended_memory_limit);

        let cells = vec![
            Cell::from(selected_mark).style(Style::default().fg(Color::Green)),
            Cell::from(rec.namespace.clone()),
            Cell::from(rec.deployment.clone()),
            Cell::from(rec.container.clone()),
            Cell::from(format!(
                "{} → {} {}",
                rec.current_cpu_request, rec.recommended_cpu_request, cpu_req_change.0
            ))
            .style(cpu_req_change.1),
            Cell::from(format!(
                "{} → {} {}",
                rec.current_cpu_limit, rec.recommended_cpu_limit, cpu_lim_change.0
            ))
            .style(cpu_lim_change.1),
            Cell::from(format!(
                "{} → {} {}",
                rec.current_memory_request, rec.recommended_memory_request, mem_req_change.0
            ))
            .style(mem_req_change.1),
            Cell::from(format!(
                "{} → {} {}",
                rec.current_memory_limit, rec.recommended_memory_limit, mem_lim_change.0
            ))
            .style(mem_lim_change.1),
        ];
        Row::new(cells).height(1)
    });

    let title = format!(
        " Resource Recommendations | Selected: {}/{} | Space: Toggle | a: All | n: None | Enter: Apply | q: Quit ",
        state.selected_indices.len(),
        output.recommendations.len()
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Percentage(10),
            Constraint::Percentage(12),
            Constraint::Percentage(10),
            Constraint::Percentage(18),
            Constraint::Percentage(15),
            Constraint::Percentage(18),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title))
    .row_highlight_style(Style::default().bg(Color::DarkGray))
    .highlight_symbol(">> ");

    // Clone the table_state to avoid borrowing issues
    let mut table_state = state.table_state.clone();
    f.render_stateful_widget(table, area, &mut table_state);
}

fn render_confirm_dialog(f: &mut ratatui::Frame, area: Rect, selected_count: usize) {
    let dialog_area = centered_rect(60, 20, area);

    let block = Block::default()
        .title(" Confirm Apply ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Apply changes to {} selected containers?", selected_count),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'y' to confirm, 'n' to cancel",
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, dialog_area);
    f.render_widget(paragraph, dialog_area);
}

fn render_input_dialog(
    f: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    input: &str,
    error: Option<&str>,
) {
    let dialog_area = centered_rect(70, 25, area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(input, Style::default().fg(Color::Cyan))),
        Line::from(""),
    ];

    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            err,
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "Press Enter to confirm, Esc to cancel",
        Style::default().fg(Color::Gray),
    )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, dialog_area);
    f.render_widget(paragraph, dialog_area);
}

fn render_progress_dialog(f: &mut ratatui::Frame, area: Rect) {
    let dialog_area = centered_rect(50, 15, area);

    let block = Block::default()
        .title(" Applying Changes ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⏳ Cloning repository...",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "⏳ Applying recommendations...",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "⏳ Creating pull request...",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please wait...",
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(Clear, dialog_area);
    f.render_widget(paragraph, dialog_area);
}

fn render_result_dialog(f: &mut ratatui::Frame, area: Rect, message: &str, pr_url: Option<&str>) {
    let dialog_area = centered_rect(70, 25, area);

    let block = Block::default()
        .title(" Result ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::Green))),
        Line::from(""),
    ];

    if let Some(url) = pr_url {
        lines.push(Line::from(Span::styled(
            "Pull Request:",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            url,
            Style::default().fg(Color::Cyan),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "Press any key to exit",
        Style::default().fg(Color::Gray),
    )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, dialog_area);
    f.render_widget(paragraph, dialog_area);
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Get change indicator and style based on comparison
fn get_change_indicator(current: &str, recommended: &str) -> (&'static str, Style) {
    if current == recommended || current == "not set" || recommended == "not set" {
        ("⚪", Style::default().fg(Color::White))
    } else {
        // Parse values for comparison
        let current_val = parse_resource_value(current);
        let recommended_val = parse_resource_value(recommended);

        if recommended_val > current_val {
            ("🟢", Style::default().fg(Color::Green))
        } else if recommended_val < current_val {
            ("🔴", Style::default().fg(Color::Red))
        } else {
            ("⚪", Style::default().fg(Color::White))
        }
    }
}

/// Parse resource value to comparable number (handles m, Mi, Gi suffixes)
fn parse_resource_value(value: &str) -> f64 {
    if value == "not set" {
        return 0.0;
    }

    // Handle CPU millicores (e.g., "100m")
    if value.ends_with('m') {
        return value.trim_end_matches('m').parse::<f64>().unwrap_or(0.0);
    }

    // Handle memory with Mi suffix
    if value.ends_with("Mi") {
        return value.trim_end_matches("Mi").parse::<f64>().unwrap_or(0.0);
    }

    // Handle memory with Gi suffix (convert to Mi)
    if value.ends_with("Gi") {
        let gi_val = value.trim_end_matches("Gi").parse::<f64>().unwrap_or(0.0);
        return gi_val * 1024.0;
    }

    // Plain number (CPU cores, convert to millicores)
    value.parse::<f64>().unwrap_or(0.0) * 1000.0
}
