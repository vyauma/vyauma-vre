use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct MetricsTracker {
    pub execution_start: Option<Instant>,
    pub execution_duration: Duration,
    pub instruction_count: u64,
    pub allocation_count: u64,
    pub task_count: u64,
}

impl MetricsTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self) {
        self.execution_start = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        if let Some(start) = self.execution_start {
            self.execution_duration = start.elapsed();
        }
    }

    pub fn record_instruction(&mut self) {
        self.instruction_count += 1;
    }

    pub fn record_allocation(&mut self) {
        self.allocation_count += 1;
    }

    pub fn record_task(&mut self) {
        self.task_count += 1;
    }

    pub fn report(&self) -> String {
        format!(
            "Execution Time: {:?}\nInstruction Count: {}\nAllocation Count: {}\nTask Count: {}",
            self.execution_duration,
            self.instruction_count,
            self.allocation_count,
            self.task_count
        )
    }
}
