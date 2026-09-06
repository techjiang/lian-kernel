# Changelog / 变更日志

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-C0778E.svg)](LICENSE-MIT)[![Rust Version](https://img.shields.io/badge/rust-1.98%2B%20(2024%20edition)-C0778E.svg)](https://www.rust-lang.org)[![Platform](https://img.shields.io/badge/platform-x86__64%20bare%20metal-C0778E.svg)](#)[![Dependencies](https://img.shields.io/badge/dependencies-0%20(zero%20crates.io)-C0778E.svg)](#)[![Version](https://img.shields.io/badge/version-0.1.0-C0778E.svg)](#)[![author](https://img.shields.io/badge/作者-科技酱-C0778E.svg)](https://docs.asoe.cn)

All notable changes to the Lian kernel will be documented in this file.

Lian 内核的所有重要变更将记录在此文件中。

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

**[English](#english) | [中文](#中文)**

---

## English

## [Unreleased]

### Planned
- Wire GDT, IDT, and PIC into `kmain` initialization sequence
- Activate physical frame allocator and 4-level paging
- Enable kernel heap allocator
- Start timer interrupts and round-robin scheduler
- Keyboard input handling via IRQ1

## [0.1.0] - 2026-09-06

### Added
- **PVH boot stub**: Custom 32-bit assembly entry point using Multiboot2 header with Xen `XEN_ELFNOTE_PHYS32_ENTRY` note for QEMU/Xen direct kernel boot
- **32-to-64-bit mode switch**: Manual PAE/long-mode/paging enable (CR4.PAE, EFER.LME, CR0.PG+PE) with 2 MiB huge page identity mapping for first 1 GiB
- **Boot GDT**: 3-segment GDT (null, 64-bit code, 64-bit data) loaded before far jump to long mode
- **VGA text output**: 80x25 text mode with 16-color CGA palette, hardware scrolling, `print!`/`println!` macros
- **16550 UART serial**: COM1 (port 0x3F8) at 38400 baud with `serial_print!`/`serial_println!` macros
- **Panic handler**: Dual-output (serial + VGA) panic diagnostics with source location
- **GDT module**: 7-segment GDT (null, kernel/user CS+DS, TSS) with `load_gdt()` and TSS support
- **IDT module**: 256-entry IDT with 16-byte interrupt gate descriptors, `InterruptFrame` struct
- **ISR stubs**: 49 interrupt service routines (vectors 0-31 exceptions, 32-47 IRQs, 128 syscall) via `global_asm!`
- **Interrupt dispatch**: `dispatch()` function routing exceptions, IRQs, and syscalls
- **8259 PIC**: Remapped to vectors 0x20-0x2F with mask/unmask support
- **8254 PIT timer**: Configurable 100 Hz tick rate with `on_timer_tick()` callback hook
- **PS/2 keyboard driver**: Set 1 scancode parsing, US QWERTY layout, 256-byte ring buffer
- **Physical frame allocator**: Bitmap-based PMM supporting up to 8 GiB (32768 bitmap words)
- **4-level paging**: Recursive PML4[511] mapping, 4 GiB identity mapping via 1 GiB huge pages, `map_page()`/`translate()`/`invlpg()`
- **Kernel heap allocator**: Free-list allocator with 256 KiB initial heap, `LockedHeap` registered as `#[global_allocator]`
- **Task struct**: `Task` with PID, name, state, priority, ticks, owned stack, `new_kernel()` constructor
- **Round-robin scheduler**: 5-tick quantum, `VecDeque` ready queue, idle task, `schedule()` and `on_timer_tick()`
- **Context switching**: `context_switch()` via `global_asm!` with callee-saved register save/restore
- **Syscall table**: 10 syscalls (exit, write, read, yield, getpid, open, close, spawn, dump, meminfo) via `int 0x80` (vector 128)
- **VFS**: `VirtualFileSystem` trait, File/Directory/CharDev node types, in-memory root filesystem
- **IPC**: Message-passing endpoints with mailbox queues, `send()`/`recv()` interface
- **HAL traits**: `ArchOps`, `MemoryOps`, `InterruptOps` for architecture abstraction
- **SpinLock**: IRQ-save/restore spinlock for single-CPU safety
- **OnceCell**: Lazy initialization primitive
- **Volatile wrapper**: `Volatile<T>` and `VolatilePtr<T>` for MMIO register access
- **Intrusive linked list**: Doubly-linked list without heap allocation
- **Bitmap allocator**: Bit-level allocator utility for frame management
- **Build system**: Makefile with build/release/iso/run/test/doc/clean/gdb targets
- **Linker script**: Custom `linker.ld` with sections at 1 MiB, PHDRS for note/text segments
- **Cargo configuration**: `x86_64-unknown-none` target, static relocation, small code model

### Verified
- `cargo build` compiles successfully (Rust 1.98+ stable, 2024 edition)
- QEMU direct boot (`-kernel`) outputs `Lian kernel: boot OK` on serial
- VGA displays welcome banner with version and architecture info
- QEMU interrupt log shows zero CPU exceptions (no triple faults, page faults, or GPFs)
- `Cargo.lock` contains zero external crate dependencies

### Known Limitations
- Subsystems (GDT, IDT, PIC, paging, heap, scheduler, etc.) are implemented but **not yet wired into `kmain`** — they compile but are not called during boot
- No user-space (ring 3) support yet
- No real filesystem driver — only VFS trait and in-memory structure
- No ACPI or APIC support
- Single-CPU only (no SMP)
- `unsafe_op_in_unsafe_fn` lint warnings present (Rust 2024 edition)

---

[Unreleased]: https://github.com/techjiang/lian-kernel/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/techjiang/lian-kernel/releases/tag/v0.1.0

---

## 中文

## [Unreleased] / 未发布

### 计划中
- 将 GDT、IDT、PIC 接入 `kmain` 初始化序列
- 激活物理帧分配器和 4 级分页
- 启用内核堆分配器
- 启动定时器中断和轮转调度器
- 通过 IRQ1 处理键盘输入

## [0.1.0] - 2026-09-06

### 新增
- **PVH 引导存根**：自定义 32 位汇编入口，使用 Multiboot2 头部和 Xen `XEN_ELFNOTE_PHYS32_ENTRY` note，支持 QEMU/Xen 直接内核引导
- **32→64 位模式切换**：手动启用 PAE/长模式/分页（CR4.PAE, EFER.LME, CR0.PG+PE），使用 2 MiB 大页身份映射前 1 GiB
- **引导 GDT**：3 段 GDT（空、64 位代码、64 位数据），在远跳转到长模式前加载
- **VGA 文本输出**：80×25 文本模式，16 色 CGA 调色板，硬件滚动，`print!`/`println!` 宏
- **16550 UART 串口**：COM1（端口 0x3F8），38400 波特率，`serial_print!`/`serial_println!` 宏
- **Panic 处理器**：双输出（串口 + VGA）panic 诊断，含源码位置
- **GDT 模块**：7 段 GDT（空、内核/用户 CS+DS、TSS），含 `load_gdt()` 和 TSS 支持
- **IDT 模块**：256 项 IDT，16 字节中断门描述符，`InterruptFrame` 结构
- **ISR 存根**：49 个中断服务例程（向量 0-31 异常，32-47 IRQ，128 系统调用），通过 `global_asm!`
- **中断分发**：`dispatch()` 函数路由异常、IRQ 和系统调用
- **8259 PIC**：重映射到向量 0x20-0x2F，支持屏蔽/取消屏蔽
- **8254 PIT 定时器**：可配置 100 Hz tick 率，`on_timer_tick()` 回调钩子
- **PS/2 键盘驱动**：Set 1 扫描码解析，美式 QWERTY 布局，256 字节环形缓冲
- **物理帧分配器**：基于位图的 PMM，最大支持 8 GiB（32768 位图字）
- **4 级分页**：递归 PML4[511] 映射，4 GiB 身份映射（1 GiB 大页），`map_page()`/`translate()`/`invlpg()`
- **内核堆分配器**：空闲链表分配器，初始 256 KiB 堆，`LockedHeap` 注册为 `#[global_allocator]`
- **任务结构**：`Task` 含 PID、名称、状态、优先级、tick 计数、拥有栈，`new_kernel()` 构造函数
- **轮转调度器**：5 tick 时间片，`VecDeque` 就绪队列，空闲任务，`schedule()` 和 `on_timer_tick()`
- **上下文切换**：`context_switch()` 通过 `global_asm!`，保存/恢复被调用者寄存器
- **系统调用表**：10 个系统调用（exit, write, read, yield, getpid, open, close, spawn, dump, meminfo），通过 `int 0x80`（向量 128）
- **VFS**：`VirtualFileSystem` trait，File/Directory/CharDev 节点类型，内存根文件系统
- **IPC**：消息传递端点，邮箱队列，`send()`/`recv()` 接口
- **HAL trait**：`ArchOps`、`MemoryOps`、`InterruptOps` 用于架构抽象
- **SpinLock**：IRQ 保存/恢复自旋锁，单 CPU 安全
- **OnceCell**：延迟初始化原语
- **Volatile 包装器**：`Volatile<T>` 和 `VolatilePtr<T>` 用于 MMIO 寄存器访问
- **侵入式链表**：无堆分配的双向链表
- **位图分配器**：位级分配器工具，用于帧管理
- **构建系统**：Makefile，含 build/release/iso/run/test/doc/clean/gdb 目标
- **链接脚本**：自定义 `linker.ld`，段位于 1 MiB，PHDRS 用于 note/text 段
- **Cargo 配置**：`x86_64-unknown-none` target，静态重定位，small code model

### 已验证
- `cargo build` 编译成功（Rust 1.98+ stable, 2024 edition）
- QEMU 直接引导（`-kernel`）串口输出 `Lian kernel: boot OK`
- VGA 显示版本和架构信息的欢迎横幅
- QEMU 中断日志显示零 CPU 异常（无三重故障、页错误或 GPF）
- `Cargo.lock` 含零外部 crate 依赖

### 已知限制
- 子系统（GDT、IDT、PIC、分页、堆、调度器等）已实现但**尚未接入 `kmain`** — 可编译但引导时不调用
- 尚无用户态（ring 3）支持
- 无真实文件系统驱动 — 仅有 VFS trait 和内存结构
- 无 ACPI 或 APIC 支持
- 仅单 CPU（无 SMP）
- 存在 `unsafe_op_in_unsafe_fn` lint 警告（Rust 2024 edition）
