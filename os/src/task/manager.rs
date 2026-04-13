//!Implementation of [`TaskManager`]

use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::BinaryHeap;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

trait Scheduler {
    fn add(&mut self, task: Arc<TaskControlBlock>);
    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>>;
}

///A array of `TaskControlBlock` that is thread-safe
pub struct FifoScheduler {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

#[allow(dead_code)]
impl FifoScheduler {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
}

/// A simple FIFO scheduler.
impl Scheduler for FifoScheduler {
    /// Add process back to ready queue
    fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

pub struct StrideTaskInfo {
    pub priority: usize,
    pub pass: usize,
    pub saved_stride: usize,
    pub is_new_task: bool,
}

impl StrideTaskInfo {
    pub const DEFAULT_PRIORITY: usize = 16;
    pub const BIG_STRIDE: usize = usize::MAX;

    pub fn new() -> Self {
        Self {
            priority: Self::DEFAULT_PRIORITY,
            pass: Self::BIG_STRIDE / Self::DEFAULT_PRIORITY,
            saved_stride: 0,
            is_new_task: true,
        }
    }

    pub fn set_priority(&mut self, priority: usize) {
        self.priority = priority;
        self.pass = Self::BIG_STRIDE / priority;
    }
}

struct StrideItem {
    stride: usize,
    task: Arc<TaskControlBlock>,
}

impl PartialEq for StrideItem {
    fn eq(&self, other: &Self) -> bool {
        self.stride == other.stride
    }
}

impl Eq for StrideItem {}

impl Ord for StrideItem {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let a = self.stride;
        let b = other.stride;
        let distance = a.wrapping_sub(b);
        if distance == 0 {
            core::cmp::Ordering::Equal
        } else if distance <= usize::MAX / 2 {
            core::cmp::Ordering::Less
        } else {
            core::cmp::Ordering::Greater
        }
    }
}

impl PartialOrd for StrideItem {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct StrideScheduler {
    ready_heap: BinaryHeap<StrideItem>,
    min_stride: usize,
}

impl StrideScheduler {
    fn new() -> Self {
        Self {
            ready_heap: BinaryHeap::new(),
            min_stride: 0,
        }
    }
}

impl Scheduler for StrideScheduler {
    fn add(&mut self, task: Arc<TaskControlBlock>) {
        let mut inner = task.inner_exclusive_access();
        let stride = if inner.stride_info.is_new_task {
            inner.stride_info.is_new_task = false;
            self.min_stride
        } else {
            inner.stride_info.saved_stride
        };
        drop(inner);
        self.ready_heap.push(StrideItem { stride, task });
    }

    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if let Some(item) = self.ready_heap.pop() {
            let task = item.task;
            let mut inner = task.inner_exclusive_access();
            inner.stride_info.saved_stride = item.stride.wrapping_add(inner.stride_info.pass);
            self.min_stride = item.stride;
            drop(inner);
            Some(task)
        } else {
            None
        }
    }
}

type CurrentScheduler = StrideScheduler;

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<CurrentScheduler> =
        unsafe { UPSafeCell::new(CurrentScheduler::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
