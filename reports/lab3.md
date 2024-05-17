```rust
pub fn sys_fork() -> isize {
    // 记录系统调用，打印当前进程的PID和调用信息
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    
    // 获取当前任务的信息
    let current_task = current_task().unwrap();
    // 调用当前任务的fork方法，创建一个新的任务
    let new_task = current_task.fork();
    // 获取新任务的PID
    let new_pid = new_task.pid.0;
    
    // 修改新任务的陷阱上下文，因为在切换之后立即返回
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // 对于子进程，fork返回值为0
    trap_cx.x[10] = 0;
    
    // 将新任务添加到调度器中
    add_task(new_task);
    
    // 返回新任务的PID
    new_pid as isize
}

```

这个函数用于实现进程的复制。它会创建当前进程的一个副本，并返回新进程的PID。在新进程中，系统调用的返回值为0。

```rust

pub fn sys_exec(path: *const u8) -> isize {
    // 记录系统调用，打印当前进程的PID和调用信息
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    
    // 获取当前用户的权限令牌
    let token = current_user_token();
    // 将路径字符串转换为Rust字符串
    let path = translated_str(token, path);
    
    // 根据路径查找应用程序数据
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        // 获取当前任务
        let task = current_task().unwrap();
        // 调用任务的exec方法，加载并执行应用程序数据
        task.exec(data);
        // 返回成功
        0
    } else {
        // 若未找到应用程序数据，则返回错误
        -1
    }
}

```

这个函数用于加载和执行一个应用程序。它通过给定的路径查找应用程序数据，并将其加载到当前进程中执行。若成功执行，则返回0；若未找到应用程序数据，则返回-1。