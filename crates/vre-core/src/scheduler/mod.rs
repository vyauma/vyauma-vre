use crate::vm::stack::Stack;
use crate::vm::vm::CallFrame;
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug)]
pub struct Scheduler {
    run_queue: VecDeque<Task>,
    next_task_id: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            run_queue: VecDeque::new(),
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
    
    pub fn iter_tasks(&self) -> impl Iterator<Item = &Task> {
        self.run_queue.iter()
    }
}
