use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use std::io;

use crate::lib::output::RecommenderOutput;

/// Display recommendations in an interactive table
pub fn display_recommendations_table(output: RecommenderOutput) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let res = run_recommendations_app(&mut terminal, output);

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
) -> io::Result<()> {
    let mut state = TableState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|f| {
            let area = f.area();

            // Create the table header
            let header_cells = [
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

            // Create table rows with change indicators
            let rows = output.recommendations.iter().map(|rec| {
                let cpu_req_change =
                    get_change_indicator(&rec.current_cpu_request, &rec.recommended_cpu_request);
                let cpu_lim_change =
                    get_change_indicator(&rec.current_cpu_limit, &rec.recommended_cpu_limit);
                let mem_req_change = get_change_indicator(
                    &rec.current_memory_request,
                    &rec.recommended_memory_request,
                );
                let mem_lim_change =
                    get_change_indicator(&rec.current_memory_limit, &rec.recommended_memory_limit);

                let cells = vec![
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
                        rec.current_memory_request,
                        rec.recommended_memory_request,
                        mem_req_change.0
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
                " Resource Recommendations | Lookback: {}h | Containers: {} | Press 'q' to quit ",
                output.metadata.lookback_hours, output.metadata.total_containers
            );

            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(10),
                    Constraint::Percentage(15),
                    Constraint::Percentage(12),
                    Constraint::Percentage(18),
                    Constraint::Percentage(18),
                    Constraint::Percentage(18),
                    Constraint::Percentage(18),
                ],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(title))
            .row_highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ");

            f.render_stateful_widget(table, area, &mut state);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = match state.selected() {
                            Some(i) => {
                                if i >= output.recommendations.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        state.select(Some(i));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = match state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    output.recommendations.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        state.select(Some(i));
                    }
                    _ => {}
                }
            }
        }
    }
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
