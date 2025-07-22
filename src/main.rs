use bcachefs_top::top_tui::{calculate_diffs, process_directory, run_tui};
use clap::Parser;
use std::{fs::read_dir, io, process::exit, thread::sleep, time::Duration};

#[derive(Parser)]
#[clap(
    name = "bcachefs-top",
    about = "A top-like tool for bcachefs statistics"
)]
struct Args {
    #[arg(short, long, default_value = "false")]
    json: bool,

    #[arg(short, long, default_value = "2")]
    time: u64,

    #[arg(short, long, default_value = "false")]
    refresh: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let bcache_dir = read_dir("/sys/fs/bcachefs/")?
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
        })
        .map(|entry| entry.path());

    let bcachefs_uuid = match bcache_dir {
        Some(dir) => dir,
        None => {
            eprintln!("No bcachefs directories found in /sys/fs/bcachefs/");
            exit(1);
        }
    };

    let bcachefs_uuid = bcachefs_uuid.to_string_lossy();
    let bcachefs_dir = format!("{bcachefs_uuid}/");
    let bcachefs_counters_dir = format!("{bcachefs_uuid}/counters/");

    if args.json && args.time > 1 {
        let results = process_directory(&bcachefs_counters_dir)?;
        sleep(Duration::from_secs(args.time));
        let second_results = process_directory(&bcachefs_counters_dir)?;

        let results = calculate_diffs(&results, &second_results);
        let json_stats = serde_json::to_string_pretty(&results)?;
        println!("{json_stats}");
        Ok(())
    } else if args.json {
        let results = process_directory(&bcachefs_counters_dir)?;
        let json_stats = serde_json::to_string_pretty(&results)?;
        println!("{json_stats}");
        Ok(())
    } else {
        run_tui(args.time, &bcachefs_dir, args.refresh)
    }
}
