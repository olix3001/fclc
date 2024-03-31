use std::{ops::Deref, path::{Path, PathBuf}, sync::{atomic::AtomicUsize, Arc}, thread::{Builder, JoinHandle}};

use hashbrown::HashMap;
use crossbeam::channel::{Receiver, Sender};
use console::style;

#[derive(Debug, Clone, Default)]
pub struct GlobalStats {
    files: HashMap<PathBuf, FileStats>
}

struct SummarizedStats {
    files: usize,
    total: usize,
    whitespace: usize,
    code: usize
}

impl GlobalStats {
    pub fn to_pretty(&self) -> String {
        use tabled::{builder::Builder, settings::{Style, Alignment}};

        let mut by_lang: HashMap<String, SummarizedStats> = HashMap::new();
        for result in self.files.values() {
            let total = result.total;
            let whitespace = result.total - result.no_whitespace;
            let code = result.code;

            if let Some(row) = by_lang.get_mut(&result.lang) {
                row.files += 1;
                row.total += total;
                row.whitespace += whitespace;
                row.code += code;
            } else {
                by_lang.insert(result.lang.clone(), SummarizedStats {
                    files: 1,
                    total,
                    whitespace,
                    code
                });
            }
        }

        let mut builder = Builder::default();

        // Header
        builder.push_record(["Language", "Files", "Total", "Whitespace", "Code"]);

        for (lang, stats) in by_lang.iter() {
            builder.push_record([
                lang.clone(),
                stats.files.to_string(),
                stats.total.to_string(),
                stats.whitespace.to_string(),
                stats.code.to_string()
            ])
        }

        builder.build()
            .with(Style::rounded())
            .to_string()
    }
}

impl Deref for GlobalStats {
    type Target = HashMap<PathBuf, FileStats>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileStats {
    path: PathBuf,
    lang: String,
    total: usize,
    no_whitespace: usize,
    code: usize
}

pub struct CounterPool {
    workers: Vec<JoinHandle<()>>,
    tasks: Sender<PathBuf>,
    results: Receiver<FileStats>,
    waiting: Arc<AtomicUsize>
}

impl CounterPool {
    pub fn with_num_workers(n: usize) -> Self {
        let (tasks_send, tasks_recv): (_, Receiver<PathBuf>) = crossbeam::channel::unbounded();
        let (results_send, results_recv) = crossbeam::channel::unbounded();
        let waiting = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();

        for _ in 0..n {
            let tasks = tasks_recv.clone();
            let results = results_send.clone();
            let waiting = Arc::clone(&waiting);
            let worker = std::thread::spawn(move || {
                while let Ok(task) = tasks.recv() {
                    match count_lines(task.as_path()) {
                        Ok(stats) => { let _ = results.send(stats).unwrap(); },
                        _ => { let _ = waiting.fetch_sub(1, std::sync::atomic::Ordering::SeqCst); }
                    }
                }
            });
            workers.push(worker);
        }

        Self {
            workers,
            tasks: tasks_send,
            results: results_recv,
            waiting
        }
    }

    pub fn queue_file(&self, file: PathBuf) {
        self.waiting.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let _ = self.tasks.send(file.canonicalize().unwrap()).unwrap();
    }

    pub fn collect_results(&self) -> GlobalStats {
        let mut global_stats = GlobalStats::default();
        loop {
            let Ok(stats) = self.results.try_recv() else {
                if self.waiting.load(std::sync::atomic::Ordering::SeqCst) == 0 { break; }
                else { continue; }
            };

            self.waiting.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            global_stats.files.insert(stats.path.clone(), stats);
        }
        global_stats
    }
}

fn count_lines(path: &Path) -> Result<FileStats, std::io::Error> {
    let mut stats = FileStats::default();
    stats.path = path.to_owned();

    // Count total lines.
    let content = std::fs::read_to_string(path)?;
    stats.total = content.lines().count();

    // Remove empty and count again.
    stats.no_whitespace = content.lines().filter(|line| !line.trim().is_empty()).count();

    // Remove all matching comments in the given lang.
    let Some(extension) = path.extension() else {
        stats.lang = style("No extension").red().to_string();
        stats.code = stats.no_whitespace;
        return Ok(stats);
    };

    let lang = crate::langs::EXTENSIONS_MAP.get(extension.to_str().unwrap());
    if let Some(lang) = lang {
        stats.lang = style(lang.name).green().to_string();
        let no_comments = lang.remove_comments(content.into());
        stats.code = no_comments.lines().filter(|line| !line.trim().is_empty()).count();
    } else {
        stats.lang = style(format!(".{}", path.extension().unwrap().to_str().unwrap())).yellow().to_string();
        stats.code = stats.no_whitespace;
    }

    Ok(stats)
}
