//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of `PID_ALLOCATOR` allocates pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod id;
pub mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
#[allow(rustdoc::private_intra_doc_links)]
mod task;

use crate::fs::{open_file, OpenFlags};
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor,
};
use crate::syscall::process::TaskInfo;
use crate::timer::get_time_ms;
// use crate::timer::get_time_ms;
// use crate::task::task::TaskInfoInner;
// use crate::syscall::process::TaskInfo;
pub use crate::mm::memory_set::{kernel_stack_position, MapPermission, MemorySet, KERNEL_SPACE};
use crate::mm::VirtPageNum;
use crate::mm::VirtAddr;
use crate::config::PAGE_SIZE;
use crate::mm::VPNRange;
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        panic!("All applications completed!");
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    // drop file descriptors
    inner.fd_table.clear();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("ch6b_initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}
 /// lab4 add 
 pub fn set_syscall_times( syscall_id: usize) {
    //let mut inner = self.current.exclusive_access();
    let cur = current_task().unwrap();
    let mut cur = cur.inner_exclusive_access();
    cur.syscall_times[syscall_id] += 1;
}
/// lab4 add
pub fn get_current_task_info(ti: *mut TaskInfo) {
    // let inner = self.inner.exclusive_access();
    // let current_id = inner.current_task;
    let cur = current_task().unwrap();
    let mut cur = cur.inner_exclusive_access();
    let trap_cx_ppn = cur.trap_cx_ppn;
    let base_size = cur.base_size;
    let task_cx = &cur.task_cx;
    let task_status = cur.task_status;
    let memory_set = cur.memory_set.clone(); // 注意需要克隆，因为 MemorySet 是 Arc 类型
    let parent = cur.parent.clone(); // 同样需要克隆
    let children = cur.children.clone(); // 同样需要克隆
    let exit_code = cur.exit_code;
    let heap_bottom = cur.heap_bottom;
    let program_brk = cur.program_brk;
    let syscall_times = cur.syscall_times;
    let start_time = cur.start_time;

    unsafe {
        *ti = TaskInfo {
            status: TaskStatus::Running,
            syscall_times,
            time: get_time_ms() - start_time,
        };
    }
}
/// lab4 add
pub fn task_map( start: usize, len: usize, port: usize) -> isize {
    if start & (PAGE_SIZE - 1) != 0 {
        // println!(
        //     "expect the start address to be aligned with a page, but get an invalid start: {:#x}",
        //     start
        // );
        return -1;
    }
    // port最低三位[x w r]，其他位必须为0
    if port > 7usize || port == 0 {
        //println!("invalid port: {:#b}", port);
        return -1;
    }

    // let mut inner = self.inner.exclusive_access();
    // let task_id = inner.current_task;
    let cur = current_task().unwrap();
    let mut cur = cur.inner_exclusive_access();
    let current_task = &mut cur;
    let memory_set = &mut current_task.memory_set;

    // check valid
    let start_vpn = VirtPageNum::from(VirtAddr(start));
    let end_vpn = VirtPageNum::from(VirtAddr(start + len).ceil());
    for vpn in start_vpn.0 .. end_vpn.0 {
        if let Some(pte) = memory_set.translate(VirtPageNum(vpn)) {
            if pte.is_valid() {

                return -1;
            }
        }
    }

// PTE_U 的语义是【用户能否访问该物理帧】
    let permission = MapPermission::from_bits((port as u8) << 1).unwrap() | MapPermission::U;
    memory_set.insert_framed_area(VirtAddr(start), VirtAddr(start+len), permission);
    0
}

pub fn current_memory_set_munmap(start_va: VirtAddr, end_va: VirtAddr) -> isize {
    // let mut inner = self.inner.exclusive_access();
    // let current_task = inner.current_task;
    let cur = current_task().unwrap();
    let mut cur = cur.inner_exclusive_access();
    cur.memory_set.remove_mapped_frames(start_va, end_va)
}

// fn get_current_id() -> usize {
//     // let inner = self.inner.exclusive_access();
//     let cur = current_task().unwrap();
//     let mut cur = cur.inner_exclusive_access();
//     cur.current_task
// }

pub fn task_munmap( start: usize, len: usize) -> isize {
    if start & (PAGE_SIZE - 1) != 0 {
        // println!(
        //     "expect the start address to be aligned with a page, but get an invalid start: {:#x}",
        //     start
        // );
        return -1;
    }

    // let mut inner = self.inner.exclusive_access();
    // let task_id = inner.current_task;
    let cur = current_task().unwrap();
    let mut cur = cur.inner_exclusive_access();
    let current_task = &mut cur;
    let memory_set = &mut current_task.memory_set;

    // check valid
    let start_vpn = VirtPageNum::from(VirtAddr(start));
    let end_vpn = VirtPageNum::from(VirtAddr(start + len).ceil());
    for vpn in start_vpn.0 .. end_vpn.0 {
        if let Some(pte) = memory_set.translate(VirtPageNum(vpn)) {
            if !pte.is_valid() {
                //println!("vpn {} is not valid before unmap", vpn);
                return -1;
            }
        }
    }

    let vpn_range = VPNRange::new(start_vpn, end_vpn);
    for vpn in vpn_range {
        memory_set.munmap(vpn);
    }

    0
}