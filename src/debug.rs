use std::{sync::{Arc, RwLock}, time::{Duration, Instant}};

use hashbrown::HashMap;

#[derive(Debug, Clone)]
pub struct DebugTimings {
    enabled: bool,
    timings: Arc<RwLock<HashMap<String, Duration>>>
}

impl DebugTimings {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            timings: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub fn start(&self, name: &str) -> DebugMeasureGuard {
        DebugMeasureGuard::start(name, &self)
    }

    pub fn table(&self) -> String {
        use tabled::{builder::Builder, settings::Style};

        let mut builder = Builder::new();
        builder.push_record(["Name", "Duration"]);

        let timings = self.timings.read().unwrap();
        for (name, time) in timings.iter() {
            builder.push_record([name.clone(), format!("{:?}", time)]);
        }

        builder.build()
            .with(Style::rounded())
            .to_string()
    }
}

pub struct DebugMeasureGuard {
    name: String,
    enabled: bool,
    start: Instant,
    timings: Arc<RwLock<HashMap<String, Duration>>>
}

impl DebugMeasureGuard {
    fn start(name: &str, timings: &DebugTimings) -> Self {
        let enabled = timings.enabled;
        let timings = timings.timings.clone();
        Self {
            name: name.to_owned(),
            enabled,
            start: Instant::now(),
            timings
        }
    }

    fn end_inner(&self) {
        if self.enabled {
            let duration = self.start.elapsed();
            self.timings.write().unwrap().insert(self.name.clone(), duration);
        }
    }

    pub fn end(self) {
        self.end_inner();
    }
}

impl Drop for DebugMeasureGuard {
    fn drop(&mut self) {
        self.end_inner()
    }
}
