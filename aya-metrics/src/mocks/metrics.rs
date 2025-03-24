// GRCOV_STOP_COVERAGE
use metrics::{Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder, SharedString, Unit};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
pub struct MockRecorder {
    counters: Arc<Mutex<HashMap<Key, Arc<AtomicU64>>>>,
}

impl MockRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_counter(&self, key: &Key) -> Option<u64> {
        self.counters
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .map(|v| v.load(Ordering::Relaxed))
    }
}

impl Recorder for MockRecorder {
    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        let key = key.clone();
        let counter = self.counters.lock().unwrap().entry(key).or_default().clone();
        Counter::from_arc(counter)
    }

    fn register_gauge(&self, _key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        Gauge::noop()
    }

    fn register_histogram(&self, _key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        Histogram::noop()
    }

    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}

    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}

    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}
}
