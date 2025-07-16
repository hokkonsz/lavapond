#[derive(Debug)]
pub struct Timer {
    start_time: std::time::Instant,
    duration: std::time::Duration,
}

impl Timer {
    /// Create a new Timer
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            duration: std::time::Duration::from_secs(0),
        }
    }

    /// Create with starting duration
    pub fn from_millis(milliseconds: u64) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            duration: std::time::Duration::from_millis(milliseconds),
        }
    }

    /// Changes timer duration
    pub fn set(&mut self, milliseconds: u64) {
        self.duration = std::time::Duration::from_millis(milliseconds);
    }

    /// Changes timer duration
    pub fn set_hz(&mut self, hertz: u64) {
        self.duration = std::time::Duration::from_millis(1000 / hertz);
    }

    /// Reset timer
    pub fn reset(&mut self) {
        self.start_time = std::time::Instant::now();
    }

    /// Returns true when Timer duration expired
    pub fn is_expired(&self) -> bool {
        if self.duration == std::time::Duration::from_secs(0) {
            return true;
        }
        // println!(
        //     "Elapsed: {:?}, Duration: {:?}",
        //     std::time::Instant::now() - self.start_time,
        //     self.duration
        // );
        std::time::Instant::now() - self.start_time >= self.duration
    }

    /// Returns true when Timer duration expired and resets Timer
    pub fn is_repeating(&mut self) -> bool {
        if self.duration == std::time::Duration::from_secs(0) {
            return true;
        }

        if std::time::Instant::now() - self.start_time >= self.duration {
            self.start_time = std::time::Instant::now();
            return true;
        }

        false
    }
}
