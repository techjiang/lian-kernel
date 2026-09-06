# Development Guide / 开发指南

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-C0778E.svg)](../LICENSE-MIT)[![Rust Version](https://img.shields.io/badge/rust-1.98%2B%20(2024%20edition)-C0778E.svg)](https://www.rust-lang.org)[![Platform](https://img.shields.io/badge/platform-x86__64%20bare%20metal-C0778E.svg)](#)[![Dependencies](https://img.shields.io/badge/dependencies-0%20(zero%20crates.io)-C0778E.svg)](#)[![Version](https://img.shields.io/badge/version-0.1.0-C0778E.svg)](#)[![author](https://img.shields.io/badge/作者-科技酱-C0778E.svg)](https://docs.asoe.cn)

This document describes the architecture, boot flow, memory layout, and development conventions of the Lian kernel.

本文档描述 Lian 内核的架构、引导流程、内存布局和开发规范。

> **Author / 作者:** [科技酱](https://docs.asoe.cn)  
> **Project / 项目:** Lian Kernel v0.1.0

**[English](#english) | [中文](#中文)**

---

## English

### Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Boot Flow (PVH)](#2-boot-flow-pvh)
3. [Memory Layout](#3-memory-layout)
4. [Page Table Setup](#4-page-table-setup)
5. [Module Architecture](#5-module-architecture)
6. [HAL Design](#6-hal-design)
7. [Interrupt Handling](#7-interrupt-handling)
8. [Scheduler & Context Switching](#8-scheduler--context-switching)
9. [Debugging Guide](#9-debugging-guide)
10. [Code Conventions](#10-code-conventions)
11. [Roadmap](#11-roadmap)

---

### 1. Architecture Overview

Lian follows a **microkernel** design:

- The kernel core provides scheduling, IPC, and address-space management.
- Device drivers and filesystem services are designed to run as separate tasks.
- Communication is via message-passing IPC (not shared memory), for isolation.

#### Crate Structure

The project is organized as a single crate with a library + binary:

```
Cargo.toml
  [lib]  name = "lian"      path = "src/lib.rs"
  [[bin]] name = "lian-kernel" path = "src/main.rs"
```

- `lib.rs` exports all kernel modules and the global allocator.
- `main.rs` contains the boot assembly stub and `kmain` entry point.

#### Zero Dependencies

The `Cargo.lock` contains no external crates. All functionality is implemented from scratch:

- No `bootloader` crate (custom PVH boot stub)
- No `x86_64` crate (custom inline assembly)
- No `spin` crate (custom `SpinLock`)
- No `volatile` crate (custom `Volatile<T>`)

---

### 2. Boot Flow (PVH)

Lian uses **PVH (Paravirtualized Hardware)** boot, which is distinct from traditional Multiboot2 bootloaders like GRUB. PVH allows QEMU and Xen to load the kernel ELF directly via the `-kernel` flag.

#### How PVH Works

1. The ELF binary contains a **Multiboot2 header** in `.multiboot2` section (for GRUB compatibility) and a **Xen ELF note** in `.note.Xen` section (for PVH entry).

2. QEMU/Xen loads the ELF and jumps to the address specified in the `XEN_ELFNOTE_PHYS32_ENTRY` note descriptor, which points to `_start`.

3. The CPU starts in **32-bit protected mode** with **no paging enabled** (CR0=0x11, EFER=0). This is the key difference from Multiboot2 which may enable some features.

#### Boot Assembly Stub (`src/main.rs`)

The boot stub is defined via `global_asm!` and follows these phases:

##### Phase 1: Stack Setup
```asm
mov esp, OFFSET stack_top    ; 64 KiB boot stack in .bss
```

##### Phase 2: Page Table Initialization
The boot stub builds a simple 3-level page table (PML4 → PDPT → PD) using **2 MiB huge pages** to identity-map the first 1 GiB of physical memory:

- PML4[0] → PDPT (Present + Writable)
- PDPT[0] → PD (Present + Writable)
- PD[0..511] → 512 × 2 MiB huge pages (Present + Writable + PS=1)

This is the **boot page table**, separate from the full 4-level paging initialized later in `memory::page::init()`.

##### Phase 3: Enable Long Mode
```asm
; Enable PAE (CR4.PAE = bit 5)
mov eax, cr4
or eax, 0x20
mov cr4, eax

; Load CR3 with PML4 address
mov eax, OFFSET pml4_table
mov cr3, eax

; Enable LME (EFER.LME = bit 8)
mov ecx, 0xC0000080
rdmsr
or eax, 0x100
wrmsr

; Enable paging + protection (CR0.PG + CR0.PE)
mov eax, cr0
or eax, 0x80000001
mov cr0, eax
```

##### Phase 4: GDT Load & Far Jump
```asm
lgdt [boot_gdt_ptr]           ; Load boot GDT (64-bit code + data segments)
ljmp $0x08, $long_mode_start  ; Far jump to 64-bit code segment
```

The boot GDT has 3 entries:
- Null descriptor
- 64-bit code segment: `0x00AF9A000000FFFF` (L=1, P=1, S=1, Type=0xA)
- 64-bit data segment: `0x00CF92000000FFFF` (P=1, S=1, Type=2)

##### Phase 5: 64-bit Segment Setup & kmain Call
```asm
long_mode_start:
    mov ax, 0x10              ; Data segment selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    xor edi, edi             ; Clear args (kmain doesn't use them yet)
    xor esi, esi

    call {kmain}             ; Enter Rust
```

#### Important Design Note

The boot stub does **not** push arguments for `kmain` across the 32→64-bit boundary. An earlier version used `push esi`/`push edi` (4 bytes each in 32-bit) but `pop rdi`/`pop rsi` (8 bytes each in 64-bit), causing a stack/page-table corruption bug. The current version zeroes `edi`/`esi` directly before calling `kmain`.

---

### 3. Memory Layout

#### ELF Section Layout (at link time)

The linker script (`linker.ld`) places everything starting at 1 MiB physical:

```
. = 1M

.note       → PT_NOTE (Xen PVH note)
.text       → PT_LOAD RX (.multiboot2 header + code)
.rodata     → PT_LOAD RX (read-only data, boot GDT)
.data       → PT_LOAD RX (mutable data)
.bss        → PT_LOAD RX (stack, page tables, BSS variables)
.got        → PT_LOAD RX (Global Offset Table)
```

#### .bss Internal Layout

```
0x102000  stack_bottom (64 KiB)
0x112000  stack_top
0x112000  pml4_table  (4 KiB)  ← same address as stack_top (aligned)
0x113000  pdpt_table  (4 KiB)
0x114000  pd_table    (4 KiB)
0x115000  ... (BSS variables: vga::ROW, etc.)
```

The boot stack (64 KiB) and boot page tables (12 KiB) are all statically allocated in `.bss`. They are used only during early boot before the full memory subsystem is initialized.

#### Boot GDT (in `.rodata`)

```
boot_gdt:
  [0] 0x0000000000000000           ; Null
  [1] 0x00AF9A000000FFFF           ; 64-bit code (L=1, Type=0x9A)
  [2] 0x00CF92000000FFFF           ; Data (Type=0x92)
boot_gdt_ptr:
  limit: boot_gdt_end - boot_gdt - 1 (= 23 = 0x17, 3 entries × 8 - 1)
  base:  boot_gdt address (.quad)
```

---

### 4. Page Table Setup

#### Boot Page Tables (Early Boot)

During the assembly boot stub, a simple identity mapping is established:

- **PML4[0]** → PDPT (Present + Writable)
- **PDPT[0]** → PD (Present + Writable)
- **PD[0..511]** → 512 × 2 MiB huge pages covering 0–1 GiB

This maps physical addresses 0 to 1 GiB to the same virtual addresses, which is sufficient for the kernel to run in its early stages.

#### Full Paging (`memory::page::init()`)

The full paging system (not yet wired into `kmain`) provides:

- **4-level paging**: PML4 → PDPT → PD → PT
- **Recursive mapping**: PML4[511] points to PML4 itself, allowing page table manipulation through recursive virtual addresses at `0xFFFF_FF80_0000_0000`
- **4 GiB identity mapping**: Using 1 GiB huge pages in PDPT[0..3]
- **`map_page()`**: Maps individual 4 KiB pages by walking the 4-level table
- **`translate()`**: Walks page tables to translate virtual → physical addresses
- **`invlpg()`**: Invalidates TLB entries

#### Page Table Entry Flags

| Flag | Bit | Value | Description |
|------|-----|-------|-------------|
| `PRESENT` | 0 | `1 << 0` | Page is present in memory |
| `WRITABLE` | 1 | `1 << 1` | Read-write access |
| `USER_ACCESS` | 2 | `1 << 2` | User-space accessible |
| `WRITE_THROUGH` | 3 | `1 << 3` | Write-through caching |
| `NO_CACHE` | 4 | `1 << 4` | Disable caching |
| `HUGE_PAGE` | 7 | `1 << 7` | 2 MiB or 1 GiB page |
| `GLOBAL` | 8 | `1 << 8` | Global page (TLB not flushed on CR3 write) |
| `NO_EXECUTE` | 63 | `1 << 63` | Execute-disable (NX bit) |

---

### 5. Module Architecture

#### Module Dependency Graph

```
lib.rs (crate root, global allocator)
├── util/         (no deps)
│   ├── volatile.rs    — Volatile<T>, VolatilePtr<T> for MMIO
│   ├── linked_list.rs — Intrusive doubly-linked list (no heap)
│   └── bitmap.rs      — Bitmap allocator (for PMM)
├── sync/         (no deps)
│   ├── spinlock.rs    — SpinLock with irq save/restore
│   └── once.rs        — OnceCell for lazy init
├── arch/
│   ├── mod.rs         — X86_64 marker type
│   └── x86_64/
│       ├── port.rs    — inb/outb/inw/outw/inl/outl
│       ├── mod.rs     — CR2/CR3, sti/cli/hlt, read_rsp
│       ├── gdt.rs     — GDT + TSS setup
│       ├── idt.rs     — 256-entry IDT, InterruptFrame
│       ├── interrupts.rs — ISR stubs + dispatch function
│       ├── pic.rs     — 8259 PIC remapping
│       ├── timer.rs   — 8254 PIT (100 Hz)
│       ├── keyboard.rs — PS/2 keyboard (Set 1)
│       └── context.rs — Context switching (global_asm!)
├── hal.rs        — HAL traits (ArchOps, MemoryOps, InterruptOps)
├── vga.rs        — VGA text-mode writer (print!/println!)
├── serial.rs     — 16550 UART (serial_print!/serial_println!)
├── panic.rs      — Panic handler
├── memory/
│   ├── mod.rs         — PAGE_SIZE, PhysAddr, VirtAddr
│   ├── frame.rs       — Bitmap frame allocator (8 GiB max)
│   ├── page.rs        — 4-level paging, recursive PML4
│   └── heap.rs        — Free-list kernel heap (256 KiB initial)
├── process/
│   ├── mod.rs
│   ├── task.rs        — Task struct, new_kernel(), dump()
│   └── scheduler.rs   — Round-robin (5-tick quantum)
├── syscall.rs    — 10 syscalls (int 0x80, vector 128)
├── fs.rs         — VFS trait, in-memory filesystem
├── ipc.rs        — Message-passing IPC, mailbox endpoints
└── driver.rs     — Driver init (timer + keyboard IRQ)
```

#### Key Patterns

- **`#[macro_export]`**: VGA and serial modules export `print!`/`println!` and `serial_print!`/`serial_println!` macros at the crate root.
- **`#[global_allocator]`**: The `LockedHeap` in `memory::heap` is registered as the global allocator in `lib.rs`.
- **`#[panic_handler]`**: The panic handler in `panic.rs` outputs to both serial and VGA.
- **`global_asm!`**: Used for boot stub (`main.rs`), ISR stubs (`interrupts.rs`), and context switching (`context.rs`).
- **`SpinLock<T>`**: Used throughout for thread-safe access to kernel data structures. Disables interrupts while locked to prevent deadlock on single-CPU systems.

---

### 6. HAL Design

The Hardware Abstraction Layer (`hal.rs`) defines three traits that each architecture must implement:

#### `ArchOps`

CPU-level operations:
- `irq_save()` / `irq_restore()` — Save/restore interrupt state
- `enable_interrupts()` / `disable_interrupts()` — sti/cli
- `halt()` — hlt instruction
- `read_cr3()` / `write_cr3()` — Page table root
- `invalidate_tlb()` — invlpg instruction

#### `MemoryOps`

Memory management operations:
- `PAGE_SIZE` — Constant (4096 on x86_64)
- `init_paging()` — Initialize paging
- `map_page()` — Map virtual to physical
- `translate()` — Virtual to physical translation
- `alloc_frame()` / `free_frame()` — Physical frame management

#### `InterruptOps`

Interrupt controller operations:
- `init_pic()` — Initialize interrupt controller
- `end_of_interrupt()` — Send EOI
- `mask_irq()` / `unmask_irq()` — Enable/disable IRQs

The current x86_64 implementation in `hal.rs` provides a partial `ArchOps` impl. `MemoryOps` and `InterruptOps` are trait definitions only (implementation pending integration).

---

### 7. Interrupt Handling

#### IDT Structure

The IDT (`arch/x86_64/idt.rs`) contains 256 entries, each a 16-byte interrupt gate descriptor:
- `offset_low` / `offset_middle` / `offset_high` — Handler address
- `selector` — Code segment (kernel CS = 0x08)
- `ist` — IST stack index (0 = no IST)
- `ty_attr` — 0x8E (Present | Ring 0 | Interrupt Gate 64-bit)

#### ISR Stubs (`arch/x86_64/interrupts.rs`)

ISR stubs are generated via `global_asm!` macros:

- **`MAKE_ISR_NOERR vector`**: Pushes a dummy error code (for exceptions that don't push one), then pushes the vector number, jumps to `isr_common`.
- **`MAKE_ISR_ERR vector`**: Uses the CPU-pushed error code, pushes the vector number, jumps to `isr_common`.

Vectors implemented:
- 0–31: CPU exceptions (#DE, #DB, NMI, #BP, ..., #SX)
- 32–47: IRQs (Timer, Keyboard, Cascade, COM1/2, ..., Primary/Secondary ATA)
- 128: System call (`int 0x80`)

#### Dispatch Function

The `dispatch()` function is called from `isr_common` with:
- `frame: *const InterruptFrame` — Saved register state
- `vector: u64` — Interrupt vector number

Dispatch logic:
1. If vector 32–47: Send EOI to PIC, route to `irq_dispatch()`
2. If vector 128: Route to `syscall::handle_syscall()`
3. If vector 0–31: Print exception name + dump frame; halt on #DF (8) and #MC (18)

#### `InterruptFrame` Layout

Matches the push order in `isr_common`:

```
[r15, r14, r13, r12, r11, r10, r9, r8,  ← pushed by stub
 rdi, rsi, rbp, rdx, rcx, rbx, rax,     ← pushed by stub
 error_code, vector,                     ← pushed by stub
 RIP, CS, RFLAGS, RSP, SS]               ← pushed by CPU
```

---

### 8. Scheduler & Context Switching

#### Scheduler (`process/scheduler.rs`)

- **Algorithm**: Round-robin preemptive
- **Quantum**: 5 timer ticks per task
- **Data structure**: `VecDeque<Box<Task>>` ready queue
- **Idle task**: Created at init, halts in a loop
- **Preemption**: `on_timer_tick()` is called from the PIT IRQ0 handler; if the current task's tick count exceeds the quantum, it is preempted

#### Task (`process/task.rs`)

```rust
pub struct Task {
    pub context: CpuContext,   // RSP + CR3
    pub id: u64,               // PID
    pub name: [u8; 16],        // Debug name
    pub state: TaskState,      // Ready | Running | Blocked | Terminated
    pub priority: u8,           // Lower = higher priority
    pub ticks: u64,            // CPU time consumed
    pub stack: Option<Box<[u8]>>, // Owned stack memory
}
```

#### Context Switching (`arch/x86_64/context.rs`)

The `context_switch` function is implemented in `global_asm!`:

```asm
context_switch:
    push rbp; push rbx; push r12; push r13; push r14; push r15
    mov [rdi], rsp      ; Save current RSP to *old_rsp
    mov rsp, rsi         ; Load new RSP
    pop r15; pop r14; pop r13; pop r12; pop rbx; pop rbp
    ret                  ; Return to new task's entry point
```

New task stacks are initialized by `init_stack()` to look as if `context_switch` had just pushed the callee-saved registers, with the return address set to the task's entry function.

---

### 9. Debugging Guide

#### Serial Output

The primary debug channel is the 16550 UART on COM1 (I/O port 0x3F8):

```rust
// In kernel code:
serial_println!("debug: value = {}", x);

// Or directly:
crate::serial::write_byte(b'X');
```

View in QEMU:
```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -serial stdio -display none
```

#### VGA Output

For visual debugging:
```rust
println!("vga message: {}", x);
```

#### QEMU Interrupt Logging

To detect CPU exceptions (page faults, GPFs, etc.):

```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -display none -d int -D target/qemu_int.log
```

Check for exceptions:
```bash
grep check_exception target/qemu_int.log
# Empty output = no exceptions = healthy
```

#### QEMU Monitor (VGA Screenshot)

```bash
# Start QEMU with QMP
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial null -display none -vga std \
  -qmp tcp:127.0.0.1:55555,server,nowait

# Take screenshot via QMP
echo '{"execute":"qmp_capabilities"}{"execute":"screendump","arguments":{"filename":"screenshot.ppm"}}' | nc 127.0.0.1 55555
```

#### GDB Debugging

```bash
# Terminal 1: Start QEMU with GDB stub (-s -S)
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -s -S

# Terminal 2: Connect GDB
gdb target/x86_64-unknown-none/debug/lian-kernel
(gdb) target remote localhost:1234
(gdb) break kmain
(gdb) continue
```

#### Panic Handler

The kernel's panic handler outputs to both serial and VGA:
```
=== LIAN KERNEL PANIC ===
  at src/main.rs:295:5
  message: ...
  halting CPU.
```

---

### 10. Code Conventions

#### Rust Edition

- **Edition**: 2024 (Rust stable 1.98+)
- **`no_std`**: The entire kernel is `#![no_std]`
- **`no_main`**: `#![no_main]` in both `lib.rs` and `main.rs`
- **`panic = "abort"`**: No unwinding (set in `Cargo.toml` profiles)

#### Unsafe Code

- `unsafe` blocks are used for: inline assembly, raw pointer dereference, MMIO, and calling architecture-specific functions.
- The 2024 edition warns about `unsafe_op_in_unsafe_fn` — unsafe operations inside unsafe functions require explicit `unsafe {}` blocks. This is a known set of warnings in the current codebase that will be progressively addressed.
- All `unsafe` functions should have `# Safety` documentation comments.

#### Assembly

- Boot stub and ISR stubs use `global_asm!` (not separate `.S` files).
- The boot stub uses Intel syntax (`.intel_syntax noprefix`) for the 32-bit section.
- ISR stubs and context switching use AT&T syntax (default).
- All assembly is embedded in Rust source via `global_asm!` with symbol substitution (e.g., `kmain = sym kmain`).

#### Naming

- Modules: `snake_case` (Rust convention)
- Types/Structs: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Functions: `snake_case`
- Static variables: `SCREAMING_SNAKE_CASE` for globals, `snake_case` for locals

#### File Organization

- One module per file (e.g., `vga.rs` for the VGA module).
- Sub-modules in directories (e.g., `arch/x86_64/`).
- Module-level documentation (`//!`) at the top of each file.
- Public functions have doc comments (`///`).

#### Build Configuration

The `.cargo/config.toml` specifies:
- `target = "x86_64-unknown-none"` (bare metal, no OS)
- `link-arg=-Tlinker.ld` (custom linker script)
- `link-arg=-no-pie` (no position-independent executable)
- `relocation-model=static` (static relocation)
- `code-model=small` (small code model; `large` causes 32-bit boot stub relocation failures)

---

### 11. Roadmap

#### Short-term (v0.2.0)
- Wire GDT, IDT, PIC into `kmain`
- Initialize paging, frame allocator, and heap at boot
- Enable timer interrupts and scheduler
- Keyboard input handling

#### Mid-term (v0.3.0–v0.5.0)
- User-space isolation (ring 3)
- Syscall table fully operational
- VFS with devfs/ramfs backends
- IPC-based driver framework

#### Long-term
- APIC/IOAPIC support
- ACPI parsing
- Real filesystem driver (ext2/FAT)
- ARM64 port (via HAL traits)
- RISC-V port (via HAL traits)
- SMP support

---

*For usage instructions, see [USAGE.md](USAGE.md).*

---

## 中文

### 目录

1. [架构概览](#1-架构概览)
2. [引导流程 (PVH)](#2-引导流程-pvh)
3. [内存布局](#3-内存布局)
4. [页表设置](#4-页表设置)
5. [模块架构](#5-模块架构)
6. [HAL 设计](#6-hal-设计)
7. [中断处理](#7-中断处理)
8. [调度器与上下文切换](#8-调度器与上下文切换)
9. [调试指南](#9-调试指南)
10. [代码规范](#10-代码规范)
11. [路线图](#11-路线图)

---

### 1. 架构概览

Lian 遵循**微内核**设计：

- 内核核心提供调度、IPC 和地址空间管理。
- 设备驱动和文件系统服务设计为独立任务运行。
- 通过消息传递 IPC 通信（非共享内存），实现隔离。

#### Crate 结构

项目组织为单个 crate，包含库和二进制：

```
Cargo.toml
  [lib]  name = "lian"      path = "src/lib.rs"
  [[bin]] name = "lian-kernel" path = "src/main.rs"
```

- `lib.rs` 导出所有内核模块和全局分配器。
- `main.rs` 包含引导汇编存根和 `kmain` 入口点。

#### 零依赖

`Cargo.lock` 不含外部 crate。所有功能从零实现：

- 无 `bootloader` crate（自定义 PVH 引导存根）
- 无 `x86_64` crate（自定义内联汇编）
- 无 `spin` crate（自定义 `SpinLock`）
- 无 `volatile` crate（自定义 `Volatile<T>`）

---

### 2. 引导流程 (PVH)

Lian 使用 **PVH（半虚拟化硬件）** 引导，与传统 Multiboot2 引导器（如 GRUB）不同。PVH 允许 QEMU 和 Xen 通过 `-kernel` 标志直接加载内核 ELF。

#### PVH 工作原理

1. ELF 二进制文件在 `.multiboot2` 段中包含 **Multiboot2 头部**（用于 GRUB 兼容），在 `.note.Xen` 段中包含 **Xen ELF note**（用于 PVH 入口）。

2. QEMU/Xen 加载 ELF 并跳转到 `XEN_ELFNOTE_PHYS32_ENTRY` note 描述符中指定的地址，指向 `_start`。

3. CPU 在 **32 位保护模式**下启动，**分页未启用**（CR0=0x11, EFER=0）。这是与 Multiboot2 的关键区别——Multiboot2 可能启用某些特性。

#### 引导汇编存根 (`src/main.rs`)

引导存根通过 `global_asm!` 定义，遵循以下阶段：

##### 阶段 1：栈设置
```asm
mov esp, OFFSET stack_top    ; .bss 中的 64 KiB 引导栈
```

##### 阶段 2：页表初始化
引导存根构建简单的 3 级页表（PML4 → PDPT → PD），使用 **2 MiB 大页**身份映射前 1 GiB 物理内存：

- PML4[0] → PDPT（Present + Writable）
- PDPT[0] → PD（Present + Writable）
- PD[0..511] → 512 × 2 MiB 大页（Present + Writable + PS=1）

这是**引导页表**，与后续在 `memory::page::init()` 中初始化的完整 4 级分页分开。

##### 阶段 3：启用长模式
```asm
; 启用 PAE (CR4.PAE = bit 5)
mov eax, cr4
or eax, 0x20
mov cr4, eax

; 用 PML4 地址加载 CR3
mov eax, OFFSET pml4_table
mov cr3, eax

; 启用 LME (EFER.LME = bit 8)
mov ecx, 0xC0000080
rdmsr
or eax, 0x100
wrmsr

; 启用分页 + 保护 (CR0.PG + CR0.PE)
mov eax, cr0
or eax, 0x80000001
mov cr0, eax
```

##### 阶段 4：加载 GDT 与远跳转
```asm
lgdt [boot_gdt_ptr]           ; 加载引导 GDT（64 位代码 + 数据段）
ljmp $0x08, $long_mode_start  ; 远跳转到 64 位代码段
```

引导 GDT 有 3 项：
- 空描述符
- 64 位代码段：`0x00AF9A000000FFFF`（L=1, P=1, S=1, Type=0xA）
- 64 位数据段：`0x00CF92000000FFFF`（P=1, S=1, Type=2）

##### 阶段 5：64 位段设置与调用 kmain
```asm
long_mode_start:
    mov ax, 0x10              ; 数据段选择子
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    xor edi, edi             ; 清除参数（kmain 暂不使用）
    xor esi, esi

    call {kmain}             ; 进入 Rust
```

#### 重要设计说明

引导存根**不**在 32→64 位边界为 `kmain` 压入参数。早期版本使用 `push esi`/`push edi`（32 位下各 4 字节），但 `pop rdi`/`pop rsi`（64 位下各 8 字节），导致栈/页表损坏 bug。当前版本直接在调用 `kmain` 前清零 `edi`/`esi`。

---

### 3. 内存布局

#### ELF 段布局（链接时）

链接脚本（`linker.ld`）将所有内容放置在物理 1 MiB 处：

```
. = 1M

.note       → PT_NOTE (Xen PVH note)
.text       → PT_LOAD RX (.multiboot2 header + 代码)
.rodata     → PT_LOAD RX (只读数据, 引导 GDT)
.data       → PT_LOAD RX (可变数据)
.bss        → PT_LOAD RX (栈, 页表, BSS 变量)
.got        → PT_LOAD RX (全局偏移表)
```

#### .bss 内部布局

```
0x102000  stack_bottom (64 KiB)
0x112000  stack_top
0x112000  pml4_table  (4 KiB)  ← 与 stack_top 相同地址（对齐）
0x113000  pdpt_table  (4 KiB)
0x114000  pd_table    (4 KiB)
0x115000  ... (BSS 变量: vga::ROW 等)
```

引导栈（64 KiB）和引导页表（12 KiB）都在 `.bss` 中静态分配，仅在早期引导阶段使用，之后由完整内存子系统接管。

#### 引导 GDT（在 `.rodata` 中）

```
boot_gdt:
  [0] 0x0000000000000000           ; 空
  [1] 0x00AF9A000000FFFF           ; 64 位代码 (L=1, Type=0x9A)
  [2] 0x00CF92000000FFFF           ; 数据 (Type=0x92)
boot_gdt_ptr:
  limit: boot_gdt_end - boot_gdt - 1 (= 23 = 0x17, 3 项 × 8 - 1)
  base:  boot_gdt 地址 (.quad)
```

---

### 4. 页表设置

#### 引导页表（早期引导）

在汇编引导存根期间，建立简单的身份映射：

- **PML4[0]** → PDPT（Present + Writable）
- **PDPT[0]** → PD（Present + Writable）
- **PD[0..511]** → 512 × 2 MiB 大页，覆盖 0–1 GiB

这将物理地址 0 到 1 GiB 映射到相同的虚拟地址，足以让内核在早期阶段运行。

#### 完整分页 (`memory::page::init()`)

完整分页系统（尚未接入 `kmain`）提供：

- **4 级分页**：PML4 → PDPT → PD → PT
- **递归映射**：PML4[511] 指向 PML4 自身，允许通过递归虚拟地址 `0xFFFF_FF80_0000_0000` 操作页表
- **4 GiB 身份映射**：使用 1 GiB 大页在 PDPT[0..3]
- **`map_page()`**：遍历 4 级表映射单个 4 KiB 页
- **`translate()`**：遍历页表翻译虚拟→物理地址
- **`invlpg()`**：使 TLB 项失效

#### 页表项标志

| 标志 | 位 | 值 | 描述 |
|------|----|-----|------|
| `PRESENT` | 0 | `1 << 0` | 页在内存中 |
| `WRITABLE` | 1 | `1 << 1` | 读写访问 |
| `USER_ACCESS` | 2 | `1 << 2` | 用户态可访问 |
| `WRITE_THROUGH` | 3 | `1 << 3` | 通写缓存 |
| `NO_CACHE` | 4 | `1 << 4` | 禁用缓存 |
| `HUGE_PAGE` | 7 | `1 << 7` | 2 MiB 或 1 GiB 页 |
| `GLOBAL` | 8 | `1 << 8` | 全局页（CR3 写入时不刷新 TLB） |
| `NO_EXECUTE` | 63 | `1 << 63` | 禁止执行 (NX bit) |

---

### 5. 模块架构

#### 模块依赖图

```
lib.rs (crate 根, 全局分配器)
├── util/         (无依赖)
│   ├── volatile.rs    — Volatile<T>, VolatilePtr<T> 用于 MMIO
│   ├── linked_list.rs — 侵入式双向链表（无堆分配）
│   └── bitmap.rs      — 位图分配器（用于 PMM）
├── sync/         (无依赖)
│   ├── spinlock.rs    — SpinLock 中断保存/恢复
│   └── once.rs        — OnceCell 延迟初始化
├── arch/
│   ├── mod.rs         — X86_64 标记类型
│   └── x86_64/
│       ├── port.rs    — inb/outb/inw/outw/inl/outl
│       ├── mod.rs     — CR2/CR3, sti/cli/hlt, read_rsp
│       ├── gdt.rs     — GDT + TSS 设置
│       ├── idt.rs     — 256 项 IDT, InterruptFrame
│       ├── interrupts.rs — ISR 存根 + 分发函数
│       ├── pic.rs     — 8259 PIC 重映射
│       ├── timer.rs   — 8254 PIT (100 Hz)
│       ├── keyboard.rs — PS/2 键盘 (Set 1)
│       └── context.rs — 上下文切换 (global_asm!)
├── hal.rs        — HAL trait (ArchOps, MemoryOps, InterruptOps)
├── vga.rs        — VGA 文本模式写入器 (print!/println!)
├── serial.rs     — 16550 UART (serial_print!/serial_println!)
├── panic.rs      — Panic 处理器
├── memory/
│   ├── mod.rs         — PAGE_SIZE, PhysAddr, VirtAddr
│   ├── frame.rs       — 位图帧分配器（最大 8 GiB）
│   ├── page.rs        — 4 级分页, 递归 PML4
│   └── heap.rs        — 空闲链表内核堆（初始 256 KiB）
├── process/
│   ├── mod.rs
│   ├── task.rs        — Task 结构, new_kernel(), dump()
│   └── scheduler.rs   — 轮转调度（5 tick 时间片）
├── syscall.rs    — 10 个系统调用 (int 0x80, 向量 128)
├── fs.rs         — VFS trait, 内存文件系统
├── ipc.rs        — 消息传递 IPC, 邮箱端点
└── driver.rs     — 驱动初始化（定时器 + 键盘 IRQ）
```

#### 关键模式

- **`#[macro_export]`**：VGA 和串口模块在 crate 根导出 `print!`/`println!` 和 `serial_print!`/`serial_println!` 宏。
- **`#[global_allocator]`**：`memory::heap` 中的 `LockedHeap` 在 `lib.rs` 中注册为全局分配器。
- **`#[panic_handler]`**：`panic.rs` 中的 panic 处理器同时输出到串口和 VGA。
- **`global_asm!`**：用于引导存根（`main.rs`）、ISR 存根（`interrupts.rs`）和上下文切换（`context.rs`）。
- **`SpinLock<T>`**：全局用于内核数据结构的线程安全访问。锁定时禁用中断，防止单 CPU 系统死锁。

---

### 6. HAL 设计

硬件抽象层（`hal.rs`）定义了三个 trait，每个架构必须实现：

#### `ArchOps`

CPU 级操作：
- `irq_save()` / `irq_restore()` — 保存/恢复中断状态
- `enable_interrupts()` / `disable_interrupts()` — sti/cli
- `halt()` — hlt 指令
- `read_cr3()` / `write_cr3()` — 页表根
- `invalidate_tlb()` — invlpg 指令

#### `MemoryOps`

内存管理操作：
- `PAGE_SIZE` — 常量（x86_64 上为 4096）
- `init_paging()` — 初始化分页
- `map_page()` — 映射虚拟到物理
- `translate()` — 虚拟到物理翻译
- `alloc_frame()` / `free_frame()` — 物理帧管理

#### `InterruptOps`

中断控制器操作：
- `init_pic()` — 初始化中断控制器
- `end_of_interrupt()` — 发送 EOI
- `mask_irq()` / `unmask_irq()` — 启用/禁用 IRQ

当前 `hal.rs` 中的 x86_64 实现提供了部分 `ArchOps` 实现。`MemoryOps` 和 `InterruptOps` 仅为 trait 定义（实现待集成）。

---

### 7. 中断处理

#### IDT 结构

IDT（`arch/x86_64/idt.rs`）包含 256 项，每项 16 字节中断门描述符：
- `offset_low` / `offset_middle` / `offset_high` — 处理器地址
- `selector` — 代码段（内核 CS = 0x08）
- `ist` — IST 栈索引（0 = 无 IST）
- `ty_attr` — 0x8E（Present | Ring 0 | 64 位中断门）

#### ISR 存根 (`arch/x86_64/interrupts.rs`)

ISR 存根通过 `global_asm!` 宏生成：

- **`MAKE_ISR_NOERR vector`**：压入虚拟错误码（不压错误码的异常使用），然后压入向量号，跳转到 `isr_common`。
- **`MAKE_ISR_ERR vector`**：使用 CPU 压入的错误码，压入向量号，跳转到 `isr_common`。

已实现向量：
- 0–31：CPU 异常（#DE, #DB, NMI, #BP, ..., #SX）
- 32–47：IRQ（定时器、键盘、级联、COM1/2, ..., 主/从 ATA）
- 128：系统调用（`int 0x80`）

#### 分发函数

`dispatch()` 函数从 `isr_common` 调用，参数：
- `frame: *const InterruptFrame` — 保存的寄存器状态
- `vector: u64` — 中断向量号

分发逻辑：
1. 向量 32–47：向 PIC 发送 EOI，路由到 `irq_dispatch()`
2. 向量 128：路由到 `syscall::handle_syscall()`
3. 向量 0–31：打印异常名称 + 转储帧；#DF (8) 和 #MC (18) 时停机

#### `InterruptFrame` 布局

与 `isr_common` 中的压栈顺序匹配：

```
[r15, r14, r13, r12, r11, r10, r9, r8,  ← 存根压入
 rdi, rsi, rbp, rdx, rcx, rbx, rax,     ← 存根压入
 error_code, vector,                     ← 存根压入
 RIP, CS, RFLAGS, RSP, SS]               ← CPU 压入
```

---

### 8. 调度器与上下文切换

#### 调度器 (`process/scheduler.rs`)

- **算法**：轮转抢占式
- **时间片**：每任务 5 个定时器 tick
- **数据结构**：`VecDeque<Box<Task>>` 就绪队列
- **空闲任务**：初始化时创建，在循环中 hlt
- **抢占**：`on_timer_tick()` 从 PIT IRQ0 处理器调用；当前任务 tick 计数超过时间片时被抢占

#### 任务 (`process/task.rs`)

```rust
pub struct Task {
    pub context: CpuContext,   // RSP + CR3
    pub id: u64,               // PID
    pub name: [u8; 16],        // 调试名称
    pub state: TaskState,      // Ready | Running | Blocked | Terminated
    pub priority: u8,           // 数字越小优先级越高
    pub ticks: u64,            // 消耗的 CPU 时间
    pub stack: Option<Box<[u8]>>, // 拥有的栈内存
}
```

#### 上下文切换 (`arch/x86_64/context.rs`)

`context_switch` 函数用 `global_asm!` 实现：

```asm
context_switch:
    push rbp; push rbx; push r12; push r13; push r14; push r15
    mov [rdi], rsp      ; 保存当前 RSP 到 *old_rsp
    mov rsp, rsi         ; 加载新 RSP
    pop r15; pop r14; pop r13; pop r12; pop rbx; pop rbp
    ret                  ; 返回到新任务的入口点
```

新任务栈通过 `init_stack()` 初始化，使其看起来像 `context_switch` 刚压入被调用者保存寄存器后的状态，返回地址设为任务的入口函数。

---

### 9. 调试指南

#### 串口输出

主要调试通道是 COM1（I/O 端口 0x3F8）上的 16550 UART：

```rust
// 在内核代码中：
serial_println!("debug: value = {}", x);

// 或直接调用：
crate::serial::write_byte(b'X');
```

在 QEMU 中查看：
```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -serial stdio -display none
```

#### VGA 输出

用于可视化调试：
```rust
println!("vga message: {}", x);
```

#### QEMU 中断日志

检测 CPU 异常（页错误、GPF 等）：

```bash
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -display none -d int -D target/qemu_int.log
```

检查异常：
```bash
grep check_exception target/qemu_int.log
# 空输出 = 无异常 = 健康
```

#### QEMU 监视器（VGA 截图）

```bash
# 启动带 QMP 的 QEMU
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial null -display none -vga std \
  -qmp tcp:127.0.0.1:55555,server,nowait

# 通过 QMP 截图
echo '{"execute":"qmp_capabilities"}{"execute":"screendump","arguments":{"filename":"screenshot.ppm"}}' | nc 127.0.0.1 55555
```

#### GDB 调试

```bash
# 终端 1：启动带 GDB stub 的 QEMU (-s -S)
qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -serial stdio -s -S

# 终端 2：连接 GDB
gdb target/x86_64-unknown-none/debug/lian-kernel
(gdb) target remote localhost:1234
(gdb) break kmain
(gdb) continue
```

#### Panic 处理器

内核的 panic 处理器同时输出到串口和 VGA：
```
=== LIAN KERNEL PANIC ===
  at src/main.rs:295:5
  message: ...
  halting CPU.
```

---

### 10. 代码规范

#### Rust Edition

- **Edition**：2024（Rust stable 1.98+）
- **`no_std`**：整个内核为 `#![no_std]`
- **`no_main`**：`lib.rs` 和 `main.rs` 均为 `#![no_main]`
- **`panic = "abort"`**：无 unwinding（在 `Cargo.toml` profiles 中设置）

#### Unsafe 代码

- `unsafe` 块用于：内联汇编、裸指针解引用、MMIO 和调用架构相关函数。
- 2024 edition 对 `unsafe_op_in_unsafe_fn` 发出警告——unsafe 函数内的 unsafe 操作需要显式 `unsafe {}` 块。这是当前代码库中已知的一组警告，将逐步处理。
- 所有 `unsafe` 函数应有 `# Safety` 文档注释。

#### 汇编

- 引导存根和 ISR 存根使用 `global_asm!`（非单独 `.S` 文件）。
- 引导存根的 32 位部分使用 Intel 语法（`.intel_syntax noprefix`）。
- ISR 存根和上下文切换使用 AT&T 语法（默认）。
- 所有汇编通过 `global_asm!` 嵌入 Rust 源码，使用符号替换（如 `kmain = sym kmain`）。

#### 命名

- 模块：`snake_case`（Rust 惯例）
- 类型/结构体：`PascalCase`
- 常量：`SCREAMING_SNAKE_CASE`
- 函数：`snake_case`
- 静态变量：全局用 `SCREAMING_SNAKE_CASE`，局部用 `snake_case`

#### 文件组织

- 每个模块一个文件（如 `vga.rs` 对应 VGA 模块）。
- 子模块放在目录中（如 `arch/x86_64/`）。
- 每个文件顶部有模块级文档（`//!`）。
- 公开函数有文档注释（`///`）。

#### 构建配置

`.cargo/config.toml` 指定：
- `target = "x86_64-unknown-none"`（裸机，无 OS）
- `link-arg=-Tlinker.ld`（自定义链接脚本）
- `link-arg=-no-pie`（非位置无关可执行文件）
- `relocation-model=static`（静态重定位）
- `code-model=small`（小代码模型；`large` 会导致 32 位引导存根重定位失败）

---

### 11. 路线图

#### 短期 (v0.2.0)
- 将 GDT、IDT、PIC 接入 `kmain`
- 启动时初始化分页、帧分配器和堆
- 启用定时器中断和调度器
- 键盘输入处理

#### 中期 (v0.3.0–v0.5.0)
- 用户态隔离（ring 3）
- 系统调用表完全可用
- VFS 支持 devfs/ramfs 后端
- 基于 IPC 的驱动框架

#### 长期
- APIC/IOAPIC 支持
- ACPI 解析
- 真实文件系统驱动（ext2/FAT）
- ARM64 移植（通过 HAL trait）
- RISC-V 移植（通过 HAL trait）
- SMP 支持

---

*使用说明请参见 [USAGE.md](USAGE.md)。*
