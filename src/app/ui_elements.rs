//==================================================
//=== Statistics
//==================================================

pub struct FrameCounter {
    start_time: std::time::Instant,
    frame_counter: u32,
    last_count: u32,
    second_last_count: u32,
}

impl FrameCounter {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            frame_counter: 0,
            last_count: 0,
            second_last_count: 1,
        }
    }

    pub fn count(&mut self) -> () {
        if (std::time::Instant::now() - self.start_time) >= std::time::Duration::from_secs(1) {
            // Update Last Frame Count
            self.second_last_count = self.last_count;
            self.last_count = self.frame_counter;

            // Reset
            self.frame_counter = 0;
            self.start_time = std::time::Instant::now();
        } else {
            self.frame_counter += 1;
        }
    }

    pub fn last_frame_count(&self) -> u32 {
        self.last_count
    }

    pub fn changed(&self) -> bool {
        self.second_last_count != self.last_count
    }
}
