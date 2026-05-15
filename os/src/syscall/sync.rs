use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);

    if current_process()
        .inner_exclusive_access()
        .deadlock_detect_enabled
    {
        if check_mutex_deadlock(mutex_id) {
            return -0xdead;
        }
    }

    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);

    if current_process()
        .inner_exclusive_access()
        .deadlock_detect_enabled
    {
        if check_semaphore_deadlock(sem_id) {
            return -0xdead;
        }
    }

    sem.down();
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    if _enabled != 0 && _enabled != 1 {
        return -1;
    }
    current_process()
        .inner_exclusive_access()
        .deadlock_detect_enabled = _enabled == 1;
    0
}

use alloc::vec;
fn check_mutex_deadlock(req_id: usize) -> bool {
    let current_process = current_process();
    let process_inner = current_process.inner_exclusive_access();
    let n = process_inner.tasks.len();
    let m = process_inner.mutex_list.len();
    let mut available = vec![0; m];
    let mut allocation = vec![vec![0; m]; n];
    let mut need = vec![vec![0; m]; n];

    for j in 0..m {
        if let Some(mutex) = &process_inner.mutex_list[j] {
            let owner = mutex.get_owner_tid();
            available[j] = if owner.is_none() { 1 } else { 0 };
            if let Some(tid) = owner {
                if tid < n {
                    allocation[tid][j] = 1;
                }
            }
            for tid in mutex.get_wait_queue_tids() {
                if tid < n {
                    need[tid][j] = 1;
                }
            }
        }
    }
    let current_tid = current_task().unwrap().get_tid();
    need[current_tid][req_id] = 1;
    run_detection_algorithm(n, m, available, allocation, need, current_tid)
}

fn check_semaphore_deadlock(req_id: usize) -> bool {
    let current_process = current_process();
    let process_inner = current_process.inner_exclusive_access();
    let n = process_inner.tasks.len();
    let m = process_inner.semaphore_list.len();
    let mut available = vec![0; m];
    let mut allocation = vec![vec![0; m]; n];
    let mut need = vec![vec![0; m]; n];

    for j in 0..m {
        if let Some(sem) = &process_inner.semaphore_list[j] {
            let inner = sem.inner.exclusive_access();
            available[j] = if inner.count >= 0 {
                inner.count as usize
            } else {
                0
            };
            for (&tid, &count) in inner.holders.iter() {
                if tid < n {
                    allocation[tid][j] = count;
                }
            }
            for task in inner.wait_queue.iter() {
                let tid = task.get_tid();
                if tid < n {
                    need[tid][j] = 1;
                }
            }
        }
    }

    let current_tid = current_task().unwrap().get_tid();
    need[current_tid][req_id] = 1;
    run_detection_algorithm(n, m, available, allocation, need, current_tid)
}

use vec::Vec;
fn run_detection_algorithm(
    n: usize,
    m: usize,
    available: Vec<usize>,
    allocation: Vec<Vec<usize>>,
    need: Vec<Vec<usize>>,
    current_tid: usize,
) -> bool {
    let mut work = available.clone();
    let mut finish = vec![false; n];

    for i in 0..n {
        if allocation[i].iter().all(|&x| x == 0) {
            finish[i] = true;
        }
    }

    loop {
        let mut found = false;
        for i in 0..n {
            if !finish[i] && (0..m).all(|j| need[i][j] <= work[j]) {
                // 模拟 i 拿到资源，运行结束，然后释放他之前就占用的资源
                for j in 0..m {
                    work[j] += allocation[i][j];
                }
                finish[i] = true;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    // println!("--- Deadlock Detected! ---");
    // println!("Available: {:?}", available);
    // println!("Allocation: {:?}", allocation);
    // println!("Need: {:?}", need);
    // println!("Current TID: {}", current_tid);
    !finish[current_tid]
}
