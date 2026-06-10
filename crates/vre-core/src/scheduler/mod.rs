use crate::vm::stack::Stack;
use crate::vm::vm::CallFrame;
use std::collections::{VecDeque, HashMap, BinaryHeap};
use std::time::{Instant, Duration};
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct Task {
    pub id: u64,
    pub ip: usize,
    pub stack: Stack,
    pub call_stack: Vec<CallFrame>,
    pub state: TaskState,
}

impl Task {
    pub fn new(id: u64, entry_ip: usize, stack_size: usize) -> Self {
        Task {
            id,
            ip: entry_ip,
            stack: Stack::new(stack_size),
            call_stack: Vec::new(),
            state: TaskState::Ready,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimerEntry {
    pub wake_time: Instant,
    pub task_id: u64,
}

impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap, so we reverse the ordering to get earliest time first
        other.wake_time.cmp(&self.wake_time)
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct Scheduler {
    run_queue: VecDeque<Task>,
    pub blocked_tasks: HashMap<u64, Task>,
    pub task_waiters: HashMap<u64, Vec<u64>>, // target_id -> list of waiting task_ids
    pub timer_queue: BinaryHeap<TimerEntry>,
    next_task_id: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            run_queue: VecDeque::new(),
            blocked_tasks: HashMap::new(),
            task_waiters: HashMap::new(),
            timer_queue: BinaryHeap::new(),
            next_task_id: 1,
        }
    }

    pub fn spawn(&mut self, entry_ip: usize, stack_size: usize) -> u64 {
        let task = Task::new(self.next_task_id, entry_ip, stack_size);
        let id = task.id;
        self.next_task_id += 1;
        self.run_queue.push_back(task);
        id
    }

    pub fn pop_next(&mut self) -> Option<Task> {
        self.run_queue.pop_front()
    }

    pub fn yield_task(&mut self, mut task: Task) {
        task.state = TaskState::Ready;
        self.run_queue.push_back(task);
    }

    pub fn block_task(&mut self, mut task: Task) {
        task.state = TaskState::Blocked;
        self.blocked_tasks.insert(task.id, task);
    }

    pub fn unblock_task(&mut self, task_id: u64) -> bool {
        if let Some(mut task) = self.blocked_tasks.remove(&task_id) {
            task.state = TaskState::Ready;
            self.run_queue.push_back(task);
            true
        } else {
            false
        }
    }

    pub fn schedule_timer(&mut self, mut task: Task, delay_ms: u64) {
        let task_id = task.id;
        self.block_task(task);
        let wake_time = Instant::now() + Duration::from_millis(delay_ms);
        self.timer_queue.push(TimerEntry { wake_time, task_id });
    }

    pub fn check_timers(&mut self) {
        let now = Instant::now();
        while let Some(entry) = self.timer_queue.peek() {
            if now >= entry.wake_time {
                let entry = self.timer_queue.pop().unwrap();
                self.unblock_task(entry.task_id);
            } else {
                break;
            }
        }
    }

    pub fn next_timer_timeout(&self) -> Option<Duration> {
        self.timer_queue.peek().map(|entry| {
            let now = Instant::now();
            if entry.wake_time > now {
                entry.wake_time.duration_since(now)
            } else {
                Duration::from_millis(0)
            }
        })
    }

    pub fn await_task(&mut self, mut task: Task, target_task_id: u64) {
        let task_id = task.id;
        self.block_task(task);
        self.task_waiters.entry(target_task_id).or_insert_with(Vec::new).push(task_id);
    }
    
    pub fn has_ready_tasks(&self) -> bool {
        !self.run_queue.is_empty()
    }

    pub fn has_active_tasks(&self) -> bool {
        !self.run_queue.is_empty() || !self.blocked_tasks.is_empty()
    }

    pub fn iter_tasks(&self) -> impl Iterator<Item = &Task> {
        self.run_queue.iter()
    }
}
