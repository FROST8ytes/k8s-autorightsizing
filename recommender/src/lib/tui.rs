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
use serde::Serialize;
use std::io;

#[derive(Debug, Clone, Serialize)]
pub struct ResourceData {
    pub deployment: String,
    pub container: String,
    pub namespace: String,
    pub cpu_request: String,
    pub cpu_limit: String,
    pub memory_request: String,
    pub memory_limit: String,
}

pub fn display_table(data: Vec<ResourceData>) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let res = run_app(&mut terminal, data);

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

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    data: Vec<ResourceData>,
) -> io::Result<()> {
    let mut state = TableState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|f| {
            let area = f.area();

            // Create the table
            let header_cells = [
                "Deployment",
                "Container",
                "Namespace",
                "CPU Request",
                "CPU Limit",
                "Memory Request",
                "Memory Limit",
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

            let rows = data.iter().map(|item| {
                let cells = vec![
                    Cell::from(item.deployment.clone()),
                    Cell::from(item.container.clone()),
                    Cell::from(item.namespace.clone()),
                    Cell::from(item.cpu_request.clone()),
                    Cell::from(item.cpu_limit.clone()),
                    Cell::from(item.memory_request.clone()),
                    Cell::from(item.memory_limit.clone()),
                ];
                Row::new(cells).height(1)
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(12),
                    Constraint::Percentage(13),
                    Constraint::Percentage(13),
                    Constraint::Percentage(16),
                    Constraint::Percentage(16),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Container Resource Requests & Limits (Press 'q' to quit) "),
            )
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
                                if i >= data.len() - 1 {
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
                                    data.len() - 1
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
