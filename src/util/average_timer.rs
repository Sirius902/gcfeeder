use std::time::{Duration, Instant};

pub struct AverageTimer {
    started: Instant,
    average: Option<Duration>,
    alpha: f64,
}

impl AverageTimer {
    pub fn start(alpha: f64) -> Result<Self, Error> {
        if alpha <= 1.0 {
            Ok(Self {
                started: Instant::now(),
                average: None,
                alpha,
            })
        } else {
            Err(Error::AlphaRange)
        }
    }

    pub fn lap(&mut self) -> Duration {
        let now = Instant::now();
        let elapsed = now - self.started;

        if let Some(average) = self.average.as_mut() {
            *average = Duration::from_secs_f64(self.alpha.mul_add(
                average.as_secs_f64(),
                (1.0 - self.alpha) * elapsed.as_secs_f64(),
            ));

            *average
        } else {
            *self.average.insert(elapsed)
        }
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("alpha should be in [0, 1]")]
    AlphaRange,
}
