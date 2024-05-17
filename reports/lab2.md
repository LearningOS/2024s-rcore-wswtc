# 通读课本

![image-20240508133839761](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508133839761.png)

操作系统确定加载到哪里，是如何实现的。

```rust
os/src/mm/heap_allocator.rs 内核动态分配内存
os/src/mm/frame_allocator.rs os以物理页帧为单位分配内存
os/src/mm/address.rs 实现虚拟地址，物理地址，虚拟页号，物理页号的各种转换
os/src/mm/page_table.rs 实现页表，包括页表项数据结构的表示，
os/src/mm/memory_set.rs 虚拟地址管理
```

页表中 页表项的索引是虚拟地址的虚拟页号

Rust中有动态的内存分配支持

地址空间，MMU自动将虚拟地址进行地址转换成物理地址。

sv39分页机制

修改satp（csr）来启用分页模式。

![image-20240508160650618](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508160650618.png)

mode为0视为物理地址

为8分页启动，39位虚存地址映射到56位物理地址。

![image-20240508160914650](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508160914650.png)

单个页面大小4KiB

![image-20240508161505212](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508161505212.png)

![image-20240508161537670](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508161537670.png)

![image-20240508161642540](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508161642540.png)

页表项，标志位

![image-20240508161951495](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508161951495.png)

![image-20240508162000649](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508162000649.png)

![image-20240508164131814](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508164131814.png)

EXT

通过satp和L2偏移找到一级页表的起始地址，然后通过L1偏移找到二级页表的起始地址，然后通过L0找到物理地址的页号

每个页表，9位索引，有2^9=512个页表项，每个页表项8字节，大小是4KiB。正好是一个物理页的大小。把一个页表放到一个物理页中，并用一个物理页号来描述它。

![image-20240508165103323](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508165103323.png)

切换任务的时候satp必须同时切换

![image-20240508165351759](C:\Users\14762\AppData\Roaming\Typora\typora-user-images\image-20240508165351759.png)

### mm

```rus
os/src/mm/heap_allocator.rs 内核动态分配内存
os/src/mm/frame_allocator.rs os以物理页帧为单位分配内存
os/src/mm/address.rs 实现虚拟地址，物理地址，虚拟页号，物理页号的各种转换
os/src/mm/page_table.rs 实现页表，包括页表项数据结构的表示，
os/src/mm/memory_set.rs 虚拟地址管理
```

sync

syscall

task

trap

# 实验

### 问题一：重写 sys_get_time 和 sys_task_info

实现一个当前进程虚拟地址到物理地址的转换方法。

### 问题二：mmap 和 munmap 匿名映射

```
fn sys_mmap(start: usize, len: usize, port: usize) -> isize
```



- syscall ID：222
- 申请长度为 len 字节的物理内存（不要求实际物理内存位置，可以随便找一块），将其映射到 start 开始的虚存，内存页属性为 port

