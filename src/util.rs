use std::time::{Duration, Instant};

macro_rules! packed_bools {
    ( ($t:ty) $($b:expr,)* ) => { {
        let mut result: $t = 0;
        let mut _i = 0_u32;

        $(
            if $b { result |= <$t>::checked_shl(1, _i).unwrap(); }
            _i += 1;
        )*

        result
    } };
}

pub(crate) use packed_bools;

pub struct AverageTimer {
    started: Instant,
    average: Option<Duration>,
    alpha: f64,
}

impl AverageTimer {
    pub fn start(alpha: f64) -> Result<Self, TimerError> {
        if alpha <= 1.0 {
            Ok(Self {
                started: Instant::now(),
                average: None,
                alpha,
            })
        } else {
            Err(TimerError::AlphaRange)
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
pub enum TimerError {
    #[error("alpha should be in [0, 1]")]
    AlphaRange,
}
