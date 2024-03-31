use std::{path::{Path, PathBuf}, time::Instant};

use clap::Parser;

use crate::counter::CounterPool;

mod counter;
mod langs;

/// FCLC (Fast Code Line Counter) is inspired by CLOC (https://github.com/AlDanial/cloc)
/// which I have used for a very long time. However CLOC is kind of old and not that fast
/// anymore. I have decided to create FCLC to replace CLOC when I need to count code lines 
/// in my projects.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Root for the counter.
    path: PathBuf
}

fn main() {
    let args = Args::parse();

    let start = Instant::now();
    let pool = CounterPool::with_num_workers(5);
    traverse_directory(args.path.as_path(), &pool);
    let stats = pool.collect_results();
    println!("Collected stats for {} files in {:?}.", indicatif::HumanCount(stats.len() as _), start.elapsed());
    println!("{}", stats.to_pretty());
}

fn traverse_directory(root: &Path, pool: &CounterPool) {
    for file in remove_exclusions(root) {
        if file.is_file() {
            pool.queue_file(file);
        } else {
            traverse_directory(file.as_path(), pool);
        }
    }
}

fn remove_exclusions(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for dir in dir.read_dir().unwrap() {
        let Ok(dir) = dir else { continue; };
        files.push(dir.path());
    }

    for lang in langs::ALL_LANGS {
        let matches = lang.match_against(&files);
        if matches {
            files.retain(|f| !lang.is_excluded(f));
        }
    }

    files
}
