use gcinput::Rumble;

macro_rules! make_patterns {
    ([ $([ $( $v:tt ),* ],)* ]) => {
        [ $( [$( pattern_value!($v), )*], )* ]
    };
}

macro_rules! pattern_value {
    (0) => {
        false
    };
    (1) => {
        true
    };
}

pub type Pattern = [bool; 6];

pub const PATTERNS: [Pattern; 7] = make_patterns!([
    [0, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 0],
    [1, 0, 0, 1, 0, 0],
    [1, 0, 1, 0, 1, 0],
    [1, 1, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1],
]);

#[derive(Debug, Default)]
pub struct PatternRumbler {
    state: PatternState,
    constant_rumble: Option<Rumble>,
}

impl PatternRumbler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_strength(&mut self, strength: u8) {
        self.state = PatternState::new(strength);
        self.constant_rumble = self.state.constant_rumble();
    }

    #[must_use]
    pub const fn constant_rumble(&self) -> Option<Rumble> {
        self.constant_rumble
    }

    #[must_use]
    pub const fn peek_rumble(&self) -> bool {
        self.state.peek_rumble()
    }

    pub fn consume_rumble(&mut self) -> bool {
        self.state.consume_rumble()
    }
}

#[derive(Debug, Default)]
struct PatternState {
    index: usize,
    poll_count: usize,
}

impl PatternState {
    pub fn new(strength: u8) -> Self {
        let index = if strength > 0 {
            1 + ((usize::from(strength) - 1) * (PATTERNS.len() - 1)) / usize::from(u8::MAX)
        } else {
            0
        };

        Self {
            index,
            poll_count: 0,
        }
    }

    #[must_use]
    pub fn constant_rumble(&self) -> Option<Rumble> {
        let mut iter = PATTERNS[self.index].iter().cloned();
        let first = iter.next().expect("pattern has at least one rumble");
        Some(first.into()).filter(|_| iter.all(|r| r == first))
    }

    #[must_use]
    pub const fn peek_rumble(&self) -> bool {
        PATTERNS[self.index][self.poll_count]
    }

    pub fn consume_rumble(&mut self) -> bool {
        let rumble = self.peek_rumble();
        self.poll_count = (self.poll_count + 1) % (PATTERNS.len() - 1);
        rumble
    }
}
