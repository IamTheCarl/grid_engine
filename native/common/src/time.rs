// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Utilities used for managing game time.

use std::{ops, time::Duration};

/// Simulation time. Is tracked in milliseconds.
/// Although you can operate on it using std::time::Duration, this struct only
/// has precision in milliseconds. That means that if you set the microseconds
/// or nanoseconds of the duration, they will be truncated from the final
/// product.
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct Time {
    time_ms: u64,
}

impl Time {
    /// Construct a new time struct from the provided time in milliseconds.
    pub fn from_ms(ms: u64) -> Time {
        Time { time_ms: ms }
    }

    // TODO display formatters.
    // TODO get delta by subtracting another
}

impl ops::Add<Duration> for Time {
    type Output = Time;

    fn add(mut self, other: Duration) -> Self {
        self.time_ms += other.as_millis() as u64;
        self
    }
}

impl ops::Sub<Duration> for Time {
    type Output = Time;

    fn sub(mut self, other: Duration) -> Self {
        self.time_ms -= other.as_millis() as u64;
        self
    }
}

impl ops::AddAssign<Duration> for Time {
    fn add_assign(&mut self, delta: Duration) {
        self.time_ms += delta.as_millis() as u64;
    }
}

impl ops::SubAssign<Duration> for Time {
    fn sub_assign(&mut self, delta: Duration) {
        self.time_ms -= delta.as_millis() as u64;
    }
}
