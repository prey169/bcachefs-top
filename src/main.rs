use bcachefs_top::{
    ioctl::process_counters,
    top_tui::{calculate_diffs, run_tui},
};
use clap::Parser;
use std::{ffi::OsString, io, thread::sleep, time::Duration};

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

    #[arg(last = true, num_args=0..=1)]
    path: Option<OsString>,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    if args.json && args.time > 2 {
        let results = process_counters(args.path.clone());
        sleep(Duration::from_secs(args.time));
        let second_results = process_counters(args.path);

        let results = calculate_diffs(&results, &second_results);
        let json_stats = serde_json::to_string_pretty(&results)?;
        println!("{json_stats}");
        Ok(())
    } else if args.json {
        let results = process_counters(args.path);
        let json_stats = serde_json::to_string_pretty(&results)?;
        println!("{json_stats}");
        Ok(())
    } else {
        run_tui(args.time, args.path)
    }
}
