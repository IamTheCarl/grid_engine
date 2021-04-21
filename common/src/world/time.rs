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
pub struct WorldTime {
    time_ms: u64,
}

impl WorldTime {
    /// Construct a new time struct from the provided time in milliseconds.
    pub fn from_ms(ms: u64) -> WorldTime {
        WorldTime { time_ms: ms }
    }

    // TODO display formatters.
    // TODO get delta by subtracting another
}

impl WorldTime {
    /// Returns the time since world creation in milliseconds.
    pub fn as_millis(&self) -> u64 {
        self.time_ms
    }
}

impl ops::Add<Duration> for WorldTime {
    type Output = WorldTime;

    fn add(self, other: Duration) -> Self {
        Self::from_ms(self.time_ms - other.as_millis() as u64)
    }
}

impl ops::Sub<Duration> for WorldTime {
    type Output = WorldTime;

    fn sub(self, other: Duration) -> Self {
        Self::from_ms(self.time_ms - other.as_millis() as u64)
    }
}

impl ops::Sub<WorldTime> for WorldTime {
    type Output = Duration;

    fn sub(self, other: WorldTime) -> Duration {
        Duration::from_millis(self.time_ms - other.time_ms)
    }
}

impl ops::AddAssign<Duration> for WorldTime {
    fn add_assign(&mut self, delta: Duration) {
        self.time_ms += delta.as_millis() as u64;
    }
}

impl ops::SubAssign<Duration> for WorldTime {
    fn sub_assign(&mut self, delta: Duration) {
        self.time_ms -= delta.as_millis() as u64;
    }
}

#[cfg(test)]
mod testing {
    // TODO test it.
}
