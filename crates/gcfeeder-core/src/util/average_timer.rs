use std::time::{Duration, Instant};

pub struct AverageTimer {
    started: Instant,
    frames: Vec<(Instant, Duration)>,
    average: Option<Duration>,
    window: Duration,
}

impl AverageTimer {
    pub fn start(window: Duration) -> Self {
        Self {
            started: Instant::now(),
            frames: Vec::new(),
            average: None,
            window,
        }
    }

    pub fn lap(&mut self) -> Duration {
        let now = Instant::now();
        let elapsed = now - self.started;

        self.frames.retain(|&(t, _)| t > now - self.window);
        self.frames.push((now, elapsed));

        let average = self
            .frames
            .iter()
            .fold(Duration::ZERO, |acc, &(_, d)| acc + d)
            / u32::try_from(self.frames.len()).unwrap();

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
