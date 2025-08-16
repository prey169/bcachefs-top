use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Layout},
    prelude::*,
    style::Color,
    widgets::{Block, Borders, Row, Table, TableState},
};
use std::{
    collections::HashMap,
    ffi::OsString,
    io::{self, stdout},
    time::{Duration, SystemTime},
};

use crate::ioctl::{BCH2_COUNTER_NAMES, process_counters};

#[derive(PartialEq)]
enum SortedBy {
    AlphabeticalAscending,
    CounterIDsAscending,
    DifferentialDescending,
}

impl SortedBy {
    fn toggle(&self) -> Self {
        match self {
            SortedBy::AlphabeticalAscending => SortedBy::CounterIDsAscending,
            SortedBy::CounterIDsAscending => SortedBy::DifferentialDescending,
            SortedBy::DifferentialDescending => SortedBy::AlphabeticalAscending,
        }
    }
    fn print(&self) -> String {
        match self {
            SortedBy::AlphabeticalAscending => "AlphabeticalAscending".to_string(),
            SortedBy::CounterIDsAscending => "CounterIDsAscending".to_string(),
            SortedBy::DifferentialDescending => "DifferentialDescending".to_string(),
        }
    }
}

pub fn calculate_diffs(
    prev: &HashMap<String, u64>,
    curr: &HashMap<String, u64>,
) -> HashMap<String, u64> {
    let mut result = HashMap::new();
    for (key, value) in curr {
        let diff = value - prev[key];
        result.insert(key.clone(), diff);
    }
    result
}

pub fn run_tui(time: u64, path: Option<OsString>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let refresh_rate = Duration::from_secs(time);
    let mut last_refresh = SystemTime::now();
    let mut force_refresh = true;

    let mut curr_stats = process_counters(path.clone());
    let starting_stats = curr_stats.clone();
    let mut previous_stats = curr_stats.clone();

    let mut sort_algo = SortedBy::AlphabeticalAscending;
    let mut alphabetical_sort: Vec<_> = curr_stats.clone().into_keys().collect();
    alphabetical_sort.sort_unstable();
    let mut scroll_offset: u16 = 0;

    let len = alphabetical_sort.len();

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code,
                modifiers,
                kind,
                ..
            }) = event::read()?
            {
                if kind == KeyEventKind::Press {
                    match (code, modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => {
                            break;
                        }
                        (KeyCode::Char('s'), _) => {
                            sort_algo = sort_algo.toggle();
                            force_refresh = true;
                        }
                        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                            scroll_offset = scroll_offset.saturating_add(1);
                            force_refresh = true;
                        }
                        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                            scroll_offset = scroll_offset.saturating_sub(1);
                            force_refresh = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_refresh.elapsed().unwrap_or(Duration::from_secs(0)) >= refresh_rate {
            previous_stats = curr_stats.clone();
            curr_stats = process_counters(path.clone());
            last_refresh = SystemTime::now();
        } else if force_refresh {
            force_refresh = false;
        } else {
            continue;
        }

        let total_diffs = calculate_diffs(&starting_stats, &curr_stats);
        let diffs = calculate_diffs(&previous_stats, &curr_stats);

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(100)])
                .split(frame.area());
            let mut stats_text = vec![Row::new(vec![
                format!("Sorted by: {}", sort_algo.print()),
                format!("{}s", time),
                "Total".to_string(),
            ])];

            let push_stats_line =
                |key: &String,
                 stats_text: &mut Vec<Row>,
                 diffs: &HashMap<String, u64>,
                 total_diffs: &HashMap<String, u64>| {
                    if total_diffs[key] != 0 {
                        stats_text.push(Row::new(vec![
                            Text::styled(format!("{key}:"), Style::default().fg(Color::Yellow)),
                            Text::from(format!("{}", diffs[key])),
                            Text::styled(
                                format!("{}", total_diffs[key]),
                                Style::default().fg(Color::Green),
                            ),
                        ]));
                    }
                };
            if sort_algo == SortedBy::AlphabeticalAscending {
                for key in &alphabetical_sort {
                    push_stats_line(key, &mut stats_text, &diffs, &total_diffs);
                }
            } else if sort_algo == SortedBy::DifferentialDescending {
                let mut diff_sort: Vec<_> = total_diffs.iter().collect();
                diff_sort.sort_by(|a, b| b.1.cmp(a.1));
                for key in &diff_sort {
                    push_stats_line(key.0, &mut stats_text, &diffs, &total_diffs);
                }
            } else if sort_algo == SortedBy::CounterIDsAscending {
                for (i, key) in BCH2_COUNTER_NAMES.iter().enumerate() {
                    if i == len {
                        break;
                    }
                    push_stats_line(&key.to_string(), &mut stats_text, &diffs, &total_diffs);
                }
            }

            let max_visible_lines = chunks[0].height.saturating_sub(2);
            let scroll_offset = scroll_offset
                .min(stats_text.len().saturating_sub(max_visible_lines as usize) as u16);

            let table = Table::default()
                .rows(stats_text)
                .widths([
                    Constraint::Length(66),
                    Constraint::Length(20),
                    Constraint::Length(20),
                ])
                .block(Block::default().borders(Borders::ALL).title("bcachefs-top"));

            let mut table_state = TableState::default();
            table_state.select(Some(scroll_offset as usize));

            frame.render_stateful_widget(table, chunks[0], &mut table_state);
        })?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
