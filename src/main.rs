use std::{path::{Path, PathBuf}, time::Instant};

use clap::Parser;

use crate::{counter::CounterPool, debug::DebugTimings};

mod counter;
mod langs;
mod debug;

/// FCLC (Fast Code Line Counter) is inspired by CLOC (https://github.com/AlDanial/cloc)
/// which I have used for a very long time. However CLOC is kind of old and not that fast
/// anymore. I have decided to create FCLC to replace CLOC when I need to count code lines 
/// in my projects.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Root for the counter.
    path: PathBuf,
    /// Include all files (eq. node_modules for node.js).
    #[arg(long)]
    include_all: bool,
    // Measure timings and include them in final output.
    #[arg(long)]
    timings: bool
}

fn main() {
    let args = Args::parse();

    let debug_timings = DebugTimings::new(args.timings);

    let start = Instant::now();

    let pool = CounterPool::with_num_workers(4);

    let pool_clone = pool.clone();
    let guard = debug_timings.start("Traverse directory tree");
    rayon::scope(move |s| {
        traverse_directory(args.path.as_path(), pool_clone, args.include_all, s);
    });
    println!("End scope");
    guard.end();

    let guard = debug_timings.start("Collect results");
    let stats = pool.collect_results();
    guard.end();

    if args.timings {
        println!("Debug timings collected: \n{}", debug_timings.table());
    }

    println!("Collected stats for {} files in {:?}.", indicatif::HumanCount(stats.len() as _), start.elapsed());
    println!("{}", stats.to_pretty());
}

fn traverse_directory(root: &Path, pool: CounterPool, skip_exclusions: bool, scope: &rayon::Scope) {
    for file in remove_exclusions(root, skip_exclusions) {
        if file.is_file() {
            pool.queue_file(file);
        } else {
            let pool = pool.clone();
            scope.spawn(move |scope| traverse_directory(file.as_path(), pool, skip_exclusions, scope));
        }
    }
}

fn remove_exclusions(dir: &Path, skip: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for dir in dir.read_dir().unwrap() {
        let Ok(dir) = dir else { continue; };
        files.push(dir.path());
    }

    if !skip {
        for lang in langs::ALL_LANGS {
            let matches = lang.match_against(&files);
            if matches {
                files.retain(|f| !lang.is_excluded(f));
            }
        }
    }

    files
}
