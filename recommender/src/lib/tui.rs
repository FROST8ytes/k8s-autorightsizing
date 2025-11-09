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
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, TableState, Wrap},
};
use std::collections::HashSet;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use url::Url;

use crate::lib::output::RecommenderOutput;
use crate::lib::recommender::ResourceRecommendation;

/// Progress update message from worker thread
#[derive(Debug, Clone)]
enum ProgressUpdate {
    Stage {
        progress: u16,
        message: String,
    },
    Complete {
        pr_url: Option<String>,
        message: String,
    },
    Error {
        message: String,
    },
}

/// Application mode
#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    BrowsingTable,
    ConfirmApply,
    InputUrl,
    InputToken,
    InputUsername,
    InputBranch,
    Applying { progress: u16, stage: String },
    ShowResult(String, Option<String>), // (message, pr_url)
}

/// Application state
struct AppState {
    table_state: TableState,
    selected_indices: HashSet<usize>,
    mode: AppMode,
    input_buffer: String,
    error_message: Option<String>,
    // Store collected values during input flow
    collected_url: Option<Url>,
    collected_token: Option<String>,
    collected_username: Option<String>,
    // Channel receiver for progress updates
    progress_rx: Option<Receiver<ProgressUpdate>>,
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
            collected_url: None,
            collected_token: None,
            collected_username: None,
            progress_rx: None,
        }
    }
}

/// Display recommendations in an interactive table
pub fn display_recommendations_table(
    output: RecommenderOutput,
    manifest_url: Option<Url>,
    git_branch: String,
    git_username: Option<String>,
    git_token: Option<String>,
) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let res = run_recommendations_app(
        &mut terminal,
        output,
        manifest_url,
        git_branch,
        git_username,
        git_token,
    );

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
    git_username: Option<String>,
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
                AppMode::InputUsername => {
                    render_table(f, area, &output, &state);
                    render_input_dialog(
                        f,
                        area,
                        "Enter Git Username (optional)",
                        &state.input_buffer,
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
                AppMode::Applying { progress, stage } => {
                    render_table(f, area, &output, &state);
                    render_progress_dialog(f, area, progress, &stage);
                }
                AppMode::ShowResult(ref message, ref pr_url) => {
                    render_table(f, area, &output, &state);
                    render_result_dialog(f, area, message, pr_url.as_deref());
                }
            }
        })?;

        // Check for progress updates from worker thread (non-blocking)
        if let Some(rx) = &state.progress_rx {
            if let Ok(update) = rx.try_recv() {
                match update {
                    ProgressUpdate::Stage { progress, message } => {
                        state.mode = AppMode::Applying {
                            progress,
                            stage: message,
                        };
                    }
                    ProgressUpdate::Complete { pr_url, message } => {
                        state.mode = AppMode::ShowResult(message, pr_url);
                        state.progress_rx = None; // Clean up channel
                    }
                    ProgressUpdate::Error { message } => {
                        state.mode = AppMode::ShowResult(message, None);
                        state.progress_rx = None; // Clean up channel
                    }
                }
            }
        }

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
                                // Always start with URL input, pre-filled if provided
                                state.mode = AppMode::InputUrl;
                                state.input_buffer = manifest_url
                                    .as_ref()
                                    .map(|u| u.to_string())
                                    .unwrap_or_default();
                                state.error_message = None;
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                state.mode = AppMode::BrowsingTable;
                            }
                            _ => {}
                        }
                    }
                    AppMode::InputUrl
                    | AppMode::InputToken
                    | AppMode::InputUsername
                    | AppMode::InputBranch => match key.code {
                        KeyCode::Enter => {
                            handle_input_submit(
                                &mut state,
                                &output,
                                &manifest_url,
                                &git_token,
                                &git_username,
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
                    },
                    AppMode::ShowResult(_, _) => {
                        // Any key returns to browsing
                        return Ok(());
                    }
                    AppMode::Applying { .. } => {
                        // No input during applying
                    }
                }
            }
        }
    }
}

fn handle_input_submit(
    state: &mut AppState,
    output: &RecommenderOutput,
    _manifest_url: &Option<Url>,
    git_token: &Option<String>,
    git_username: &Option<String>,
    git_branch: &str,
) {
    match &state.mode {
        AppMode::InputUrl => {
            // Validate URL
            match Url::parse(&state.input_buffer) {
                Ok(url) => {
                    state.collected_url = Some(url);
                    state.mode = AppMode::InputToken;
                    // Pre-fill with existing token if provided via CLI
                    state.input_buffer = git_token.clone().unwrap_or_default();
                    state.error_message = None;
                }
                Err(e) => {
                    state.error_message = Some(format!("Invalid URL: {}", e));
                }
            }
        }
        AppMode::InputToken => {
            // Token is optional, store it
            state.collected_token = if state.input_buffer.is_empty() {
                None
            } else {
                Some(state.input_buffer.clone())
            };
            // Move to username input
            state.mode = AppMode::InputUsername;
            state.input_buffer = git_username.clone().unwrap_or_default();
            state.error_message = None;
        }
        AppMode::InputUsername => {
            // Username is optional, store it
            state.collected_username = if state.input_buffer.is_empty() {
                None
            } else {
                Some(state.input_buffer.clone())
            };
            // Move to branch input
            state.mode = AppMode::InputBranch;
            state.input_buffer = git_branch.to_string();
            state.error_message = None;
        }
        AppMode::InputBranch => {
            // All inputs collected, spawn worker thread
            let branch = state.input_buffer.clone();

            if let Some(url) = &state.collected_url {
                // Get selected recommendations
                let selected_recommendations: Vec<ResourceRecommendation> = state
                    .selected_indices
                    .iter()
                    .filter_map(|&i| output.recommendations.get(i).cloned())
                    .collect();

                // Spawn worker thread with apply task
                let rx = spawn_apply_worker(
                    url.clone(),
                    branch,
                    state.collected_username.clone(),
                    state.collected_token.clone(),
                    selected_recommendations,
                );

                // Store receiver and transition to Applying mode
                state.progress_rx = Some(rx);
                state.mode = AppMode::Applying {
                    progress: 0,
                    stage: "Initializing...".to_string(),
                };
            }
        }
        _ => {}
    }
}

/// Spawn a worker thread that performs the apply operation
fn spawn_apply_worker(
    url: Url,
    branch: String,
    username: Option<String>,
    token: Option<String>,
    recommendations: Vec<ResourceRecommendation>,
) -> Receiver<ProgressUpdate> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // Create tokio runtime in worker thread
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(ProgressUpdate::Error {
                    message: format!("Failed to create async runtime: {}", e),
                });
                return;
            }
        };

        // Run async apply operation
        rt.block_on(async {
            use crate::lib::config::UpdaterConfig;
            use crate::lib::updater::ManifestUpdater;

            // Send initial progress
            let _ = tx.send(ProgressUpdate::Stage {
                progress: 10,
                message: "Initializing updater...".to_string(),
            });

            // Create updater config
            let config = match UpdaterConfig::new(url.clone(), token.clone(), username) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ProgressUpdate::Error {
                        message: format!("Failed to create updater config: {}", e),
                    });
                    return;
                }
            };

            let _ = tx.send(ProgressUpdate::Stage {
                progress: 20,
                message: "Creating updater...".to_string(),
            });

            // Create updater
            let mut updater = match ManifestUpdater::new(config) {
                Ok(u) => u,
                Err(e) => {
                    let _ = tx.send(ProgressUpdate::Error {
                        message: format!("Failed to create updater: {}", e),
                    });
                    return;
                }
            };

            let _ = tx.send(ProgressUpdate::Stage {
                progress: 30,
                message: "Cloning repository...".to_string(),
            });

            // Apply and create PR
            match updater.apply_and_create_pr(&branch, &recommendations).await {
                Ok((new_branch, _commit_sha, pr_url)) => {
                    let _ = tx.send(ProgressUpdate::Stage {
                        progress: 90,
                        message: "Finalizing...".to_string(),
                    });

                    let message = format!(
                        "Successfully applied {} recommendation(s) to branch '{}'",
                        recommendations.len(),
                        new_branch
                    );

                    let _ = tx.send(ProgressUpdate::Complete { pr_url, message });
                }
                Err(e) => {
                    let _ = tx.send(ProgressUpdate::Error {
                        message: format!("Failed to apply recommendations: {}", e),
                    });
                }
            }
        });
    });

    rx
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
                "{} → {}",
                rec.current_cpu_request, rec.recommended_cpu_request,
            ))
            .style(cpu_req_change),
            Cell::from(format!(
                "{} → {}",
                rec.current_cpu_limit, rec.recommended_cpu_limit,
            ))
            .style(cpu_lim_change),
            Cell::from(format!(
                "{} → {}",
                rec.current_memory_request, rec.recommended_memory_request,
            ))
            .style(mem_req_change),
            Cell::from(format!(
                "{} → {}",
                rec.current_memory_limit, rec.recommended_memory_limit,
            ))
            .style(mem_lim_change),
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

fn render_progress_dialog(f: &mut ratatui::Frame, area: Rect, progress: u16, stage: &str) {
    let dialog_area = centered_rect(60, 20, area);

    // Split the dialog area into sections
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Gauge
            Constraint::Length(2), // Stage message
            Constraint::Min(0),    // Padding
        ])
        .split(dialog_area);

    // Clear background
    f.render_widget(Clear, dialog_area);

    // Render title block
    let title_block = Block::default()
        .title(" Applying Changes ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    f.render_widget(title_block, dialog_area);

    // Render progress gauge
    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(
            Style::default()
                .fg(Color::Green)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .percent(progress)
        .label(format!("{}%", progress));

    f.render_widget(gauge, chunks[1]);

    // Render stage message
    let stage_text = vec![
        Line::from(""),
        Line::from(Span::styled(stage, Style::default().fg(Color::Yellow))),
    ];
    let stage_paragraph = Paragraph::new(stage_text).alignment(Alignment::Center);
    f.render_widget(stage_paragraph, chunks[2]);
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
fn get_change_indicator(current: &str, recommended: &str) -> Style {
    if current == recommended || current == "not set" || recommended == "not set" {
        Style::default().fg(Color::White)
    } else {
        // Parse values for comparison
        let current_val = parse_resource_value(current);
        let recommended_val = parse_resource_value(recommended);

        if recommended_val > current_val {
            Style::default().fg(Color::Green)
        } else if recommended_val < current_val {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
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
