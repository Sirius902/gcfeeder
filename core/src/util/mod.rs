pub use average_timer::AverageTimer;

pub mod average_timer;
pub mod recent_channel;

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
