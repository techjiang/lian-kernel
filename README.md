# Lian Kernel

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-C0778E.svg)](LICENSE-MIT)[![Rust Version](https://img.shields.io/badge/rust-1.98%2B%20(2024%20edition)-C0778E.svg)](https://www.rust-lang.org)[![Platform](https://img.shields.io/badge/platform-x86__64%20bare%20metal-C0778E.svg)](#)[![Dependencies](https://img.shields.io/badge/dependencies-0%20(zero%20crates.io)-C0778E.svg)](#)[![Version](https://img.shields.io/badge/version-0.1.0-C0778E.svg)](#)[![author](https://img.shields.io/badge/作者-科技酱-C0778E.svg)](https://docs.asoe.cn)

> 一个用 Rust 从零开始编写的微内核——零外部依赖，裸机 x86_64，并带有 HAL 抽象层，以便将来支持 ARM64/RISC-V。
>
> A microkernel written in Rust from scratch — zero external dependencies, bare-metal x86_64 with a HAL abstraction layer for future ARM64/RISC-V support.

**Author:** [科技酱](https://docs.asoe.cn) | **Version:** 0.1.0 | **License:** MIT OR Apache-2.0

**[English](#english) | [中文](#中文)**

---

## English

### Overview

Lian is a from-scratch microkernel targeting x86_64 bare metal, written entirely in Rust with **zero crates.io dependencies**. It boots via PVH (Paravirtualized Hardware) protocol through a Multiboot2 header with a Xen-style `XEN_ELFNOTE_PHYS32_ENTRY` note, giving it broad compatibility with QEMU, Xen, and modern hypervisors.

The project follows a microkernel design philosophy: the kernel core provides scheduling, IPC, and address-space management, while device drivers and filesystem services are designed to run as separate tasks communicating via message-passing IPC.

#### Current Status (v0.1.0)

The boot path is fully working: the kernel enters 64-bit long mode, initializes serial output and VGA text display, and successfully halts in an idle loop. All core subsystems listed below have been implemented in code and compile cleanly, but most are **not yet wired into the `kmain` initialization sequence** — they exist as ready-to-integrate modules and will be progressively activated in upcoming releases.

| Status | Description |
|--------|-------------|
| **Working** | PVH boot, 32→64-bit mode switch, page tables (2 MiB huge pages), VGA text output, 16550 UART serial, panic handler |
| **Implemented, pending integration** | GDT/TSS, IDT + 49 ISR stubs, 8259 PIC, 8254 PIT timer, PS/2 keyboard, physical frame allocator, 4-level paging, kernel heap, round-robin scheduler, task switching, VFS, IPC, syscalls |
| **Planned** | User-space isolation, APIC/IOAPIC, ACPI, real filesystem driver, ARM64/RISC-V ports |

### Core Subsystems

| Subsystem | Module | Description |
|-----------|--------|-------------|
| **Boot** | `main.rs` | PVH/Multiboot2 header, 64 KiB boot stack, 32→64-bit assembly stub |
| **VGA** | `vga.rs` | 80x25 text-mode, 16 colors, scrolling, `print!`/`println!` macros |
| **Serial** | `serial.rs` | 16550 UART COM1 (0x3F8), `serial_print!`/`serial_println!` macros |
| **GDT** | `arch/x86_64/gdt.rs` | 7-segment GDT (null, kernel/user CS+DS, TSS) |
| **IDT** | `arch/x86_64/idt.rs` | 256-entry IDT, interrupt gates |
| **ISRs** | `arch/x86_64/interrupts.rs` | 49 ISR stubs + syscall (vector 128) via `global_asm!` |
| **PIC** | `arch/x86_64/pic.rs` | 8259 PIC remapped to 0x20-0x2F |
| **PIT** | `arch/x86_64/timer.rs` | 8254 PIT at 100 Hz, tick counter |
| **Keyboard** | `arch/x86_64/keyboard.rs` | PS/2 Set 1 scancode, US layout, 256-byte ring buffer |
| **PMM** | `memory/frame.rs` | Bitmap frame allocator (8 GiB max, 32768 words) |
| **Paging** | `memory/page.rs` | 4-level paging, recursive PML4[511], 4 GiB identity-mapped (1 GiB huge pages) |
| **Heap** | `memory/heap.rs` | Free-list kernel heap, 256 KiB initial, grows on demand |
| **Scheduler** | `process/scheduler.rs` | Round-robin, 5-tick quantum, idle task |
| **Task** | `process/task.rs` | Kernel task struct, `context_switch` via inline asm |
| **Syscalls** | `syscall.rs` | 10 syscalls: exit/write/read/yield/getpid/open/close/spawn/dump/meminfo |
| **VFS** | `fs.rs` | VFS trait, File/Directory/CharDev nodes, root filesystem |
| **IPC** | `ipc.rs` | Microkernel message passing, mailbox endpoints |
| **HAL** | `hal.rs` | `ArchOps`/`MemoryOps`/`InterruptOps` traits |
| **Sync** | `sync/` | `SpinLock` (irq save/restore), `OnceCell` |
| **Utils** | `util/` | `Volatile`, intrusive `LinkedList`, `Bitmap` |

### Design Principles

- **Microkernel**: IPC-first design — drivers and services are designed to run as tasks, communicating via message passing
- **Zero external dependencies**: 100% Rust stable, no `bootloader`/`x86_64`/`spin` crates
- **HAL abstraction**: `hal.rs` traits isolate architecture-specific code; ARM64/RISC-V porting requires only trait impl
- **Memory safety**: `unsafe` blocks are minimal and documented; `SpinLock` for concurrency

### Quick Start

#### Prerequisites

- **Rust** (stable 1.98+, with 2024 edition support): [https://rustup.rs](https://rustup.rs)
- **Rust target**: `x86_64-unknown-none`
- **QEMU** (optional, for boot testing): [https://www.qemu.org](https://www.qemu.org)

#### Install Rust Target

```bash
rustup target add x86_64-unknown-none
```

#### Build

```bash
cargo build
```

The build configuration in `.cargo/config.toml` automatically targets `x86_64-unknown-none` and passes the custom linker script.

#### Run in QEMU

```bash
# Direct boot (fastest, no bootloader needed)
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio

# With VGA display
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio -vga std

# Headless (serial only)
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio -display none
```

#### Expected Output

Serial:
```
Lian kernel: boot OK
```

VGA (text mode 80x25):
```
==================================================
 Lian Kernel v0.1.0
 Architecture: x86_64 (PVH boot, Long Mode)
==================================================
```

#### Using Make

```bash
make build      # Compile (debug)
make release    # Compile (release/optimized)
make run-direct # Boot in QEMU (direct multiboot)
make iso        # Build bootable ISO (requires grub-mkrescue/xorriso)
make run        # Boot from ISO in QEMU
make test       # Run kernel tests
make doc        # Generate documentation
make clean      # Remove all build artifacts
make help       # Show all targets
```

### Architecture

```
+---------------------------------------------------------+
|                    User-space tasks                      |
|  +----------+  +----------+  +----------+  +---------+  |
|  | FS server|  | Driver   |  | Network  |  | App     |  |
|  | (ipc)    |  | server   |  | server   |  | process |  |
|  +-----+----+  +----+-----+  +----+-----+  +----+----+  |
|        +----------+---------+----------+               |
|                     IPC message passing                 |
+---------------------------------------------------------+
|                     Lian microkernel                    |
|  +---------+ +----------+ +--------+ +---------------+  |
|  |Scheduler| | Memory    | | IPC    | | Syscall       |  |
|  | (RR)    | | (PMM+VM) | | (MBX)  | | (vec 128)     |  |
|  +---------+ +----------+ +--------+ +---------------+  |
|  +-----------------------------------------------------+|
|  |              HAL (hardware abstraction)             ||
|  |         x86_64 | ARM64 (planned) | RISC-V (planned)||
|  +-----------------------------------------------------+|
+---------------------------------------------------------+
|                    Hardware (bare metal)                 |
+---------------------------------------------------------+
```

### Boot Sequence

The kernel uses PVH (Paravirtualized Hardware) boot, which starts in 32-bit protected mode without paging enabled:

1. **PVH entry** — QEMU/Xen loads the ELF kernel and jumps to the `XEN_ELFNOTE_PHYS32_ENTRY` address in 32-bit protected mode
2. **Stack setup** — 64 KiB boot stack at `stack_bottom`
3. **Page tables** — PML4, PDPT, and PD are populated statically in `.bss`:
   - PML4[0] -> PDPT (identity-mapped, 4 GiB)
   - PDPT[0] -> PD (512 entries, each mapping a 2 MiB huge page)
   - 512 PD entries cover the first 1 GiB of physical memory
4. **Enable long mode** — Set CR4.PAE, EFER.LME, CR0.PG + CR0.PE
5. **Load GDT** — Boot GDT with 64-bit code segment (0x08) and data segment (0x10)
6. **Far jump** — `ljmp $0x08, $long_mode_start` enters 64-bit long mode
7. **Segment setup** — Load DS, ES, FS, GS, SS with data selector
8. **Call kmain** — Enter the Rust entry point

Once in `kmain`, the current v0.1.0 initialization is:
1. **Serial init** — UART COM1 configured at 38400 baud
2. **VGA clear** — Screen initialized with color
3. **Welcome banner** — Version and architecture info printed
4. **Halt loop** — `hlt` in infinite loop

Subsequent releases will progressively wire in GDT, IDT, PIC, paging, heap, scheduler, and other subsystems.

### Source Tree

```
src/
+-- main.rs              # PVH boot stub + kmain entry point
+-- lib.rs               # Crate root, global allocator, IRQ dispatch
+-- vga.rs               # VGA text output, print!/println! macros
+-- serial.rs            # 16550 UART, serial_print!/serial_println! macros
+-- panic.rs             # Panic handler (serial + VGA diagnostic)
+-- syscall.rs           # Syscall table (vector 128)
+-- fs.rs                # VFS trait + in-memory filesystem
+-- ipc.rs               # Message-passing IPC endpoints
+-- driver.rs            # Driver initialization
+-- hal.rs               # Hardware abstraction traits
+-- arch/
|   +-- mod.rs           # Architecture marker types
|   +-- x86_64/
|       +-- mod.rs       # CR2/CR3, sti/cli/hlt
|       +-- port.rs      # inb/outb/inl/outl I/O port helpers
|       +-- gdt.rs       # GDT + TSS setup
|       +-- idt.rs       # 256-entry IDT, InterruptFrame
|       +-- interrupts.rs # ISR stubs (global_asm!) + dispatch
|       +-- pic.rs       # 8259 PIC remapping
|       +-- timer.rs     # 8254 PIT (100 Hz)
|       +-- keyboard.rs  # PS/2 keyboard driver
|       +-- context.rs   # Context switching (global_asm!)
+-- memory/
|   +-- mod.rs           # Type aliases, constants
|   +-- frame.rs         # Bitmap physical frame allocator
|   +-- page.rs          # 4-level paging, recursive mapping
|   +-- heap.rs          # Free-list kernel heap
+-- process/
|   +-- mod.rs           # Process module
|   +-- task.rs          # Task struct, stack init
|   +-- scheduler.rs     # Round-robin scheduler
+-- sync/
|   +-- mod.rs           # Sync module
|   +-- spinlock.rs      # SpinLock (irq save/restore)
|   +-- once.rs          # OnceCell (lazy init)
+-- util/
    +-- mod.rs           # Util module
    +-- volatile.rs      # Volatile<T> wrapper for MMIO
    +-- linked_list.rs   # Intrusive doubly linked list
    +-- bitmap.rs        # Bitmap allocator
```

### Syscalls

| # | Name | Description |
|---|------|-------------|
| 0 | exit | Terminate current task |
| 1 | write | Write to file descriptor |
| 2 | read | Read from file descriptor |
| 3 | yield | Yield CPU to scheduler |
| 4 | getpid | Get current task PID |
| 5 | open | Open a VFS node |
| 6 | close | Close a file descriptor |
| 7 | spawn | Spawn a new kernel task |
| 8 | dump | Dump task list (debug) |
| 9 | meminfo | Get memory statistics |

### Multi-Architecture Support

The HAL in `hal.rs` defines three traits:

- `ArchOps`: CPU-level operations (sti/cli/hlt, CR3 read/write, TLB flush)
- `MemoryOps`: Page size, paging init, map/translate, frame alloc/free
- `InterruptOps`: PIC init, EOI, mask/unmask

Porting to ARM64 or RISC-V requires:
1. Implement the HAL traits for the new architecture
2. Provide boot assembly stub in `arch/<arch>/`
3. Replace GDT/IDT/PIC with arch-equivalent (e.g., GIC for ARM64)

### Configuration Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Package manifest, profile settings (panic=abort) |
| `.cargo/config.toml` | Target triple, linker script, code-model settings |
| `linker.ld` | ELF linker script (sections at 1 MiB, PHDRS, DISCARD) |
| `Makefile` | Build system (build, run, iso, test, doc, clean) |

### Debugging

#### Serial Debug

The kernel outputs diagnostic messages to COM1 (0x3F8). View serial output in QEMU:

```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -serial stdio -display none
```

#### QEMU Interrupt Log

To check for CPU exceptions (page faults, GPFs, etc.):

```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -display none -d int -D target/qemu_int.log
```

If `target/qemu_int.log` is empty or contains no `check_exception` entries, the kernel is running without faults.

#### GDB Debugging

```bash
# Start QEMU with GDB stub
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -s -S

# In another terminal
gdb target/x86_64-unknown-none/debug/lian-kernel
(gdb) target remote localhost:1234
```

### Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### License

Licensed under either of:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

### Acknowledgments

- The Rust `no_std` and bare-metal ecosystem communities
- OSDev Wiki for x86 hardware reference
- The Multiboot2 and PVH specifications

### Links

- **Author:** [科技酱](https://docs.asoe.cn)
- **Documentation:** [https://docs.asoe.cn](https://docs.asoe.cn)
- **Issue Tracker:** [GitHub Issues](https://github.com/techjiang/lian-kernel/issues)

### Stats

- **Language:** Rust (stable, 2024 edition)
- **External dependencies:** 0 (zero crates.io)
- **Source files:** 34
- **Lines of code:** ~3,850
- **Target:** x86_64 bare metal (PVH boot)
- **Architecture:** Microkernel (IPC-based services)

---

## 中文

### 概览

Lian 是一个从零开始编写的 x86_64 裸机微内核，完全使用 Rust 语言实现，**零 crates.io 外部依赖**。它通过 PVH（半虚拟化硬件）协议引导，使用 Multiboot2 头部和 Xen 风格的 `XEN_ELFNOTE_PHYS32_ENTRY` note，兼容 QEMU、Xen 和现代虚拟化平台。

项目遵循微内核设计理念：内核核心提供调度、IPC 和地址空间管理，设备驱动和文件系统服务设计为独立任务运行，通过消息传递 IPC 通信。

#### 当前状态 (v0.1.0)

引导路径完全可用：内核进入 64 位长模式，初始化串口输出和 VGA 文本显示，在空闲循环中成功挂起。以下所有核心子系统已在代码中实现并编译通过，但大多数**尚未接入 `kmain` 初始化序列** — 它们作为待集成模块存在，将在后续版本中逐步激活。

| 状态 | 描述 |
|------|------|
| **已工作** | PVH 引导、32→64 位模式切换、页表（2 MiB 大页）、VGA 文本输出、16550 UART 串口、panic 处理器 |
| **已实现，待集成** | GDT/TSS、IDT + 49 个 ISR 存根、8259 PIC、8254 PIT 定时器、PS/2 键盘、物理帧分配器、4 级分页、内核堆、轮转调度器、任务切换、VFS、IPC、系统调用 |
| **计划中** | 用户态隔离、APIC/IOAPIC、ACPI、真实文件系统驱动、ARM64/RISC-V 移植 |

### 核心子系统

| 子系统 | 模块 | 描述 |
|--------|------|------|
| **引导** | `main.rs` | PVH/Multiboot2 头部、64 KiB 引导栈、32→64 位汇编存根 |
| **VGA** | `vga.rs` | 80×25 文本模式、16 色、滚动、`print!`/`println!` 宏 |
| **串口** | `serial.rs` | 16550 UART COM1 (0x3F8)、`serial_print!`/`serial_println!` 宏 |
| **GDT** | `arch/x86_64/gdt.rs` | 7 段 GDT（空、内核/用户 CS+DS、TSS） |
| **IDT** | `arch/x86_64/idt.rs` | 256 项 IDT、中断门 |
| **ISR** | `arch/x86_64/interrupts.rs` | 49 个 ISR 存根 + 系统调用（向量 128），通过 `global_asm!` |
| **PIC** | `arch/x86_64/pic.rs` | 8259 PIC 重映射到 0x20-0x2F |
| **PIT** | `arch/x86_64/timer.rs` | 8254 PIT，100 Hz，tick 计数器 |
| **键盘** | `arch/x86_64/keyboard.rs` | PS/2 Set 1 扫描码、美式布局、256 字节环形缓冲 |
| **PMM** | `memory/frame.rs` | 位图帧分配器（最大 8 GiB，32768 字） |
| **分页** | `memory/page.rs` | 4 级分页、递归 PML4[511]、4 GiB 身份映射（1 GiB 大页） |
| **堆** | `memory/heap.rs` | 空闲链表内核堆、初始 256 KiB、按需扩展 |
| **调度器** | `process/scheduler.rs` | 轮转调度、5 tick 时间片、空闲任务 |
| **任务** | `process/task.rs` | 内核任务结构、`context_switch` 内联汇编 |
| **系统调用** | `syscall.rs` | 10 个系统调用：exit/write/read/yield/getpid/open/close/spawn/dump/meminfo |
| **VFS** | `fs.rs` | VFS trait、File/Directory/CharDev 节点、根文件系统 |
| **IPC** | `ipc.rs` | 微内核消息传递、邮箱端点 |
| **HAL** | `hal.rs` | `ArchOps`/`MemoryOps`/`InterruptOps` trait |
| **同步** | `sync/` | `SpinLock`（中断保存/恢复）、`OnceCell` |
| **工具** | `util/` | `Volatile`、侵入式 `LinkedList`、`Bitmap` |

### 设计原则

- **微内核**：IPC 优先设计 — 驱动和服务设计为任务运行，通过消息传递通信
- **零外部依赖**：100% Rust stable，不使用 `bootloader`/`x86_64`/`spin` 等 crate
- **HAL 抽象**：`hal.rs` trait 隔离架构相关代码；ARM64/RISC-V 移植只需实现 trait
- **内存安全**：`unsafe` 块最小化且有文档；`SpinLock` 保证并发安全

### 快速上手

#### 前置条件

- **Rust**（stable 1.98+，支持 2024 edition）：[https://rustup.rs](https://rustup.rs)
- **Rust target**：`x86_64-unknown-none`
- **QEMU**（可选，用于启动测试）：[https://www.qemu.org](https://www.qemu.org)

#### 安装 Rust Target

```bash
rustup target add x86_64-unknown-none
```

#### 构建

```bash
cargo build
```

`.cargo/config.toml` 中的构建配置自动指定 `x86_64-unknown-none` target 并传入自定义链接脚本。

#### 在 QEMU 中运行

```bash
# 直接引导（最快，无需引导器）
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio

# 带 VGA 显示
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio -vga std

# 无头模式（仅串口）
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio -display none
```

#### 预期输出

串口：
```
Lian kernel: boot OK
```

VGA（80×25 文本模式）：
```
==================================================
 Lian Kernel v0.1.0
 Architecture: x86_64 (PVH boot, Long Mode)
==================================================
```

#### 使用 Make

```bash
make build      # 编译（debug）
make release    # 编译（release/优化）
make run-direct # 在 QEMU 中直接引导
make iso        # 构建可引导 ISO（需要 grub-mkrescue/xorriso）
make run        # 从 ISO 引导
make test       # 运行内核测试
make doc        # 生成文档
make clean      # 清除所有构建产物
make help       # 显示所有目标
```

### 架构图

```
+---------------------------------------------------------+
|                    用户态任务                              |
|  +----------+  +----------+  +----------+  +---------+  |
|  | FS server|  | Driver   |  | Network  |  | App     |  |
|  | (ipc)    |  | server   |  | server   |  | process |  |
|  +-----+----+  +----+-----+  +----+-----+  +----+----+  |
|        +----------+---------+----------+               |
|                     IPC 消息传递                          |
+---------------------------------------------------------+
|                     Lian 微内核                           |
|  +---------+ +----------+ +--------+ +---------------+  |
|  |Scheduler| | Memory    | | IPC    | | Syscall       |  |
|  | (RR)    | | (PMM+VM) | | (MBX)  | | (vec 128)     |  |
|  +---------+ +----------+ +--------+ +---------------+  |
|  +-----------------------------------------------------+|
|  |              HAL (硬件抽象层)                        ||
|  |         x86_64 | ARM64 (计划) | RISC-V (计划)        ||
|  +-----------------------------------------------------+|
+---------------------------------------------------------+
|                    硬件 (裸机)                            |
+---------------------------------------------------------+
```

### 引导序列

内核使用 PVH（半虚拟化硬件）引导，在 32 位保护模式下启动，分页未启用：

1. **PVH 入口** — QEMU/Xen 加载 ELF 内核并跳转到 `XEN_ELFNOTE_PHYS32_ENTRY` 地址，在 32 位保护模式下执行
2. **栈设置** — 在 `stack_bottom` 处设置 64 KiB 引导栈
3. **页表** — PML4、PDPT 和 PD 在 `.bss` 中静态填充：
   - PML4[0] -> PDPT（身份映射，4 GiB）
   - PDPT[0] -> PD（512 项，每项映射一个 2 MiB 大页）
   - 512 个 PD 项覆盖前 1 GiB 物理内存
4. **启用长模式** — 设置 CR4.PAE、EFER.LME、CR0.PG + CR0.PE
5. **加载 GDT** — 引导 GDT 含 64 位代码段（0x08）和数据段（0x10）
6. **远跳转** — `ljmp $0x08, $long_mode_start` 进入 64 位长模式
7. **段设置** — 用数据选择子加载 DS、ES、FS、GS、SS
8. **调用 kmain** — 进入 Rust 入口点

进入 `kmain` 后，当前 v0.1.0 的初始化步骤为：
1. **串口初始化** — UART COM1 配置为 38400 波特率
2. **VGA 清屏** — 带颜色初始化屏幕
3. **欢迎横幅** — 打印版本和架构信息
4. **挂起循环** — `hlt` 无限循环

后续版本将逐步接入 GDT、IDT、PIC、分页、堆、调度器等子系统。

### 源码树

```
src/
+-- main.rs              # PVH 引导存根 + kmain 入口
+-- lib.rs               # Crate 根，全局分配器，IRQ 分发
+-- vga.rs               # VGA 文本输出，print!/println! 宏
+-- serial.rs            # 16550 UART，serial_print!/serial_println! 宏
+-- panic.rs             # Panic 处理器（串口 + VGA 诊断）
+-- syscall.rs           # 系统调用表（向量 128）
+-- fs.rs                # VFS trait + 内存文件系统
+-- ipc.rs               # 消息传递 IPC 端点
+-- driver.rs            # 驱动初始化
+-- hal.rs               # 硬件抽象 trait
+-- arch/
|   +-- mod.rs           # 架构标记类型
|   +-- x86_64/
|       +-- mod.rs       # CR2/CR3, sti/cli/hlt
|       +-- port.rs      # inb/outb/inl/outl I/O 端口辅助
|       +-- gdt.rs       # GDT + TSS 设置
|       +-- idt.rs       # 256 项 IDT, InterruptFrame
|       +-- interrupts.rs # ISR 存根 (global_asm!) + 分发
|       +-- pic.rs       # 8259 PIC 重映射
|       +-- timer.rs     # 8254 PIT (100 Hz)
|       +-- keyboard.rs  # PS/2 键盘驱动
|       +-- context.rs   # 上下文切换 (global_asm!)
+-- memory/
|   +-- mod.rs           # 类型别名，常量
|   +-- frame.rs         # 位图物理帧分配器
|   +-- page.rs          # 4 级分页，递归映射
|   +-- heap.rs          # 空闲链表内核堆
+-- process/
|   +-- mod.rs           # 进程模块
|   +-- task.rs          # 任务结构，栈初始化
|   +-- scheduler.rs     # 轮转调度器
+-- sync/
|   +-- mod.rs           # 同步模块
|   +-- spinlock.rs      # SpinLock (中断保存/恢复)
|   +-- once.rs          # OnceCell (延迟初始化)
+-- util/
    +-- mod.rs           # 工具模块
    +-- volatile.rs      # Volatile<T> MMIO 包装器
    +-- linked_list.rs   # 侵入式双向链表
    +-- bitmap.rs        # 位图分配器
```

### 系统调用

| # | 名称 | 描述 |
|---|------|------|
| 0 | exit | 终止当前任务 |
| 1 | write | 写入文件描述符 |
| 2 | read | 读取文件描述符 |
| 3 | yield | 让出 CPU 给调度器 |
| 4 | getpid | 获取当前任务 PID |
| 5 | open | 打开 VFS 节点 |
| 6 | close | 关闭文件描述符 |
| 7 | spawn | 创建新内核任务 |
| 8 | dump | 转储任务列表（调试） |
| 9 | meminfo | 获取内存统计 |

### 多架构支持

`hal.rs` 中的 HAL 定义了三个 trait：

- `ArchOps`：CPU 级操作（sti/cli/hlt、CR3 读写、TLB 刷新）
- `MemoryOps`：页大小、分页初始化、映射/翻译、帧分配/释放
- `InterruptOps`：PIC 初始化、EOI、屏蔽/取消屏蔽

移植到 ARM64 或 RISC-V 需要：
1. 为新架构实现 HAL trait
2. 在 `arch/<arch>/` 中提供引导汇编存根
3. 用架构等价物替换 GDT/IDT/PIC（如 ARM64 的 GIC）

### 配置文件

| 文件 | 用途 |
|------|------|
| `Cargo.toml` | 包清单、profile 设置（panic=abort） |
| `.cargo/config.toml` | target triple、链接脚本、code-model 设置 |
| `linker.ld` | ELF 链接脚本（段位于 1 MiB、PHDRS、DISCARD） |
| `Makefile` | 构建系统（build, run, iso, test, doc, clean） |

### 调试

#### 串口调试

内核向 COM1 (0x3F8) 输出诊断信息。在 QEMU 中查看串口输出：

```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -serial stdio -display none
```

#### QEMU 中断日志

检查 CPU 异常（页错误、GPF 等）：

```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -display none -d int -D target/qemu_int.log
```

如果 `target/qemu_int.log` 为空或不含 `check_exception` 条目，表示内核运行无故障。

#### GDB 调试

```bash
# 启动 QEMU 并挂起等待 GDB
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -s -S

# 在另一个终端连接
gdb target/x86_64-unknown-none/debug/lian-kernel
(gdb) target remote localhost:1234
```

### 贡献

欢迎贡献。请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

### 许可证

根据以下任一许可证授权：
- MIT 许可证（[LICENSE-MIT](LICENSE-MIT)）
- Apache 许可证 2.0 版（[LICENSE-APACHE](LICENSE-APACHE)）

由你选择。

### 致谢

- Rust `no_std` 和裸机生态社区
- OSDev Wiki 的 x86 硬件参考
- Multiboot2 和 PVH 规范

### 链接

- **作者：** [科技酱](https://docs.asoe.cn)
- **文档：** [https://docs.asoe.cn](https://docs.asoe.cn)
- **问题追踪：** [GitHub Issues](https://github.com/techjiang/lian-kernel/issues)

### 统计

- **语言：** Rust（stable, 2024 edition）
- **外部依赖：** 0（零 crates.io）
- **源文件：** 34
- **代码行数：** ~3,850
- **目标平台：** x86_64 裸机（PVH 引导）
- **架构：** 微内核（基于 IPC 的服务）
