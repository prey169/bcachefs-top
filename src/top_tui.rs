use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Layout},
    prelude::Stylize,
    style::Color,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, read_dir},
    io::{self, BufRead, BufReader, stdout},
    time::{Duration, SystemTime},
};

#[derive(PartialEq)]
enum SortedBy {
    AlphabeticalAscending,
    DifferentialDescending,
}

impl SortedBy {
    fn toggle(&self) -> Self {
        match self {
            SortedBy::AlphabeticalAscending => SortedBy::DifferentialDescending,
            SortedBy::DifferentialDescending => SortedBy::AlphabeticalAscending,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct DataSize {
    file_name: String,
    value: u64,
}

pub fn calculate_diffs(
    prev: &HashMap<String, u128>,
    curr: &HashMap<String, u128>,
) -> HashMap<String, u128> {
    let mut result = HashMap::new();
    for (key, value) in curr {
        let diff = value - prev[key];
        result.insert(key.clone(), diff);
    }
    result
}

fn parse_data_size(line: &str) -> Option<u128> {
    let re = Regex::new(r"(\d+\.?\d*)\s*(KiB|MiB|GiB|TiB|PiB|EiB|ZiB|YiB)?").unwrap();
    if let Some(captures) = re.captures(line) {
        let value: f64 = captures.get(1)?.as_str().parse().ok()?;
        let bytes = if let Some(unit) = captures.get(2) {
            let multiplier = match unit.as_str() {
                "KiB" => 1024u128,
                "MiB" => 1024u128.pow(2),
                "GiB" => 1024u128.pow(3),
                "TiB" => 1024u128.pow(4),
                "PiB" => 1024u128.pow(5),
                "EiB" => 1024u128.pow(6),
                "ZiB" => 1024u128.pow(7),
                "YiB" => 1024u128.pow(8),
                _ => 1,
            };
            (value * multiplier as f64) as u128
        } else {
            value as u128
        };
        Some(bytes)
    } else {
        None
    }
}

pub fn process_directory(dir: &str) -> io::Result<HashMap<String, u128>> {
    let mut results = HashMap::new();

    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if let Ok(Some(bytes)) = process_file(path.to_str().unwrap_or("")) {
                    results.insert(file_name.to_string(), bytes);
                }
            }
        }
    }

    Ok(results)
}
fn process_file(path: &str) -> io::Result<Option<u128>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.contains("since mount") {
            if let Some(bytes) = parse_data_size(&line) {
                return Ok(Some(bytes));
            }
        }
    }
    Ok(None)
}

pub fn run_tui(time: u64, bcachefs_dir: &str, refresh: bool) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let refresh_rate = Duration::from_secs(time);
    let mut last_refresh = SystemTime::now();
    let mut force_refresh = true;

    let bcachefs_counters_dir = format!("{bcachefs_dir}/counters/");
    let mut curr_stats = process_directory(&bcachefs_counters_dir)?;
    let mut starting_stats = curr_stats.clone();

    let mut sort_algo = SortedBy::AlphabeticalAscending;
    let mut alphabetical_sort: Vec<_> = curr_stats.clone().into_keys().collect();
    alphabetical_sort.sort_unstable();
    let mut scroll_offset: u16 = 0;

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
            if refresh {
                starting_stats = curr_stats.clone();
            }
            curr_stats = process_directory(&bcachefs_counters_dir)?;
            last_refresh = SystemTime::now();
        } else if force_refresh {
            force_refresh = false;
        } else {
            continue;
        }

        let diffs = calculate_diffs(&starting_stats, &curr_stats);

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(100)])
                .split(frame.area());
            let mut stats_text = vec![];

            let push_stats_line =
                |key: &String,
                 stats_text: &mut Vec<Line>,
                 curr_stats: &HashMap<String, u128>,
                 diffs: &HashMap<String, u128>| {
                    stats_text.push(Line::from(vec![
                        format!("{key}: ").fg(Color::Yellow),
                        format!("{}", curr_stats[key]).into(),
                        format!(" (Diff: +{})", diffs[key]).fg(Color::Green),
                    ]));
                };

            if sort_algo == SortedBy::AlphabeticalAscending {
                for key in &alphabetical_sort {
                    push_stats_line(key, &mut stats_text, &curr_stats, &diffs);
                }
            } else if sort_algo == SortedBy::DifferentialDescending {
                let mut diff_sort: Vec<_> = diffs.iter().collect();
                diff_sort.sort_by(|a, b| b.1.cmp(a.1));
                for key in &diff_sort {
                    push_stats_line(key.0, &mut stats_text, &curr_stats, &diffs);
                }
            }

            let max_visible_lines = chunks[0].height.saturating_sub(2);
            scroll_offset = scroll_offset
                .min(stats_text.len().saturating_sub(max_visible_lines as usize) as u16);

            let paragraph = Paragraph::new(stats_text)
                .block(Block::default().borders(Borders::ALL).title("bcachefs-top"))
                .scroll((scroll_offset, 0));
            frame.render_widget(paragraph, chunks[0]);
        })?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
