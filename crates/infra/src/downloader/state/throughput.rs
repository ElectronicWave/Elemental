use std::time::{Duration, Instant};

#[derive(Debug)]
pub(super) struct ThroughputMeter {
    window_started_at: Instant,
    current_window_bytes: usize,
    value: usize,
}

impl Default for ThroughputMeter {
    fn default() -> Self {
        Self {
            window_started_at: Instant::now(),
            current_window_bytes: 0,
            value: 0,
        }
    }
}

impl ThroughputMeter {
    pub(super) fn record(&mut self, bytes: usize, now: Instant) {
        self.roll_window(now);
        self.current_window_bytes = self.current_window_bytes.saturating_add(bytes);
    }

    pub(super) fn value(&mut self, now: Instant) -> usize {
        self.roll_window(now);
        self.value
    }

    fn roll_window(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.window_started_at);
        if elapsed < Duration::from_secs(1) {
            return;
        }

        let rate = self.current_window_bytes as f64 / elapsed.as_secs_f64();
        self.value = rate as usize;
        self.current_window_bytes = 0;
        self.window_started_at = now;
    }
}
