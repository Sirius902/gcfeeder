use std::time::{Duration, Instant};

pub struct AverageTimer {
    started: Instant,
    last_lap: Instant,
    frames: Vec<(Instant, Duration)>,
    average: Option<Duration>,
    window: Duration,
}

impl AverageTimer {
    pub fn start(window: Duration) -> Self {
        let now = Instant::now();
        Self {
            started: now,
            last_lap: now,
            frames: Vec::new(),
            average: None,
            window,
        }
    }

    pub fn lap(&mut self) -> Duration {
        let now = Instant::now();

        self.frames.retain(|&(t, _)| t > now.checked_sub(self.window).unwrap());
        self.frames.push((now, now - self.last_lap));

        let average = self
            .frames
            .iter()
            .fold(Duration::ZERO, |acc, &(_, d)| acc + d)
            / u32::try_from(self.frames.len()).unwrap();

        self.last_lap = now;
        *self.average.insert(average)
    }

    pub fn read(&self) -> Duration {
        self.started.elapsed()
    }

    pub const fn read_avg(&self) -> Option<Duration> {
        self.average
    }

    pub fn reset(&mut self) {
        self.started = Instant::now();
    }
}
