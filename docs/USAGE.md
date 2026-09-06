# Usage Guide / 使用指南

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-C0778E.svg)](../LICENSE-MIT)[![Rust Version](https://img.shields.io/badge/rust-1.98%2B%20(2024%20edition)-C0778E.svg)](https://www.rust-lang.org)[![Platform](https://img.shields.io/badge/platform-x86__64%20bare%20metal-C0778E.svg)](#)[![Dependencies](https://img.shields.io/badge/dependencies-0%20(zero%20crates.io)-C0778E.svg)](#)[![Version](https://img.shields.io/badge/version-0.1.0-C0778E.svg)](#)[![author](https://img.shields.io/badge/作者-科技酱-C0778E.svg)](https://docs.asoe.cn)

This document covers environment setup, building, running, and debugging the Lian kernel.

本文档涵盖 Lian 内核的环境搭建、构建、运行和调试。

> **Author / 作者:** [科技酱](https://docs.asoe.cn)  
> **Project / 项目:** Lian Kernel v0.1.0

**[English](#english) | [中文](#中文)**

---

## English

### Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Install the Toolchain](#2-install-the-toolchain)
3. [Build the Kernel](#3-build-the-kernel)
4. [Run in QEMU](#4-run-in-qemu)
5. [Expected Output](#5-expected-output)
6. [Make Targets Reference](#6-make-targets-reference)
7. [Debugging](#7-debugging)
8. [Troubleshooting](#8-troubleshooting)
9. [Platform-Specific Notes](#9-platform-specific-notes)

---

### 1. Prerequisites

| Tool | Required | Minimum Version | Purpose |
|------|----------|-----------------|---------|
| **Rust** | Yes | 1.98+ (stable, 2024 edition) | Compile the kernel |
| **Rust target `x86_64-unknown-none`** | Yes | — | Bare-metal target triple |
| **QEMU** | Recommended | 7.0+ (`qemu-system-x86_64`) | Boot and test the kernel |
| **xorriso** or **grub-mkrescue** | Optional | — | Build bootable ISO images |
| **GDB** | Optional | 10+ | Source-level debugging |
| **make** | Optional | 4.0+ | Use the Makefile targets |

> If you only want to build and run the kernel directly (without the Makefile), you only need Rust + QEMU.

---

### 2. Install the Toolchain

#### 2.1 Install Rust

Install via [rustup](https://rustup.rs):

**Linux / macOS / WSL:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

**Windows:**
Download and run `rustup-init.exe` from [https://rustup.rs](https://rustup.rs), or install via winget:
```bash
winget install Rustlang.Rustup
```

Verify installation:
```bash
rustc --version
cargo --version
```

> **Note:** The project requires Rust 2024 edition, which is supported in Rust 1.98 and later. If your version is older, run `rustup update`.

#### 2.2 Add the Bare-Metal Target

```bash
rustup target add x86_64-unknown-none
```

This target is a "bare metal" triple — it has no operating system, no standard library, and no runtime. The `.cargo/config.toml` is pre-configured to use it automatically.

#### 2.3 Install QEMU

**Linux (Debian/Ubuntu):**
```bash
sudo apt install qemu-system-x86
```

**Linux (Fedora):**
```bash
sudo dnf install qemu-system-x86-core
```

**macOS (Homebrew):**
```bash
brew install qemu
```

**Windows (winget):**
```bash
winget install qemu.qemu
```

Verify:
```bash
qemu-system-x86_64 --version
```

#### 2.4 (Optional) Install ISO Build Tools

Only needed if you want to build bootable ISO images (`make iso`):

**Linux (Debian/Ubuntu):**
```bash
sudo apt install xorriso grub-pc-bin
```

**macOS:**
```bash
brew install xorriso
```

**Windows:** ISO creation is not natively supported on Windows. Use the direct `-kernel` boot method instead (see [Section 4.1](#41-direct-boot-recommended)).

#### 2.5 (Optional) Install GDB

**Linux:** `sudo apt install gdb`  
**macOS:** `brew install gdb`  
**Windows:** Install via MinGW or use the GDB bundled with the Rust toolchain (`rust-gdb`).

---

### 3. Build the Kernel

#### 3.1 Debug Build

```bash
cargo build
```

Or explicitly:
```bash
cargo build --target x86_64-unknown-none
```

The output binary is at:
```
target/x86_64-unknown-none/debug/lian-kernel
```

> The `.cargo/config.toml` automatically sets the target, linker script (`linker.ld`), and code model. You do not need to pass any extra flags.

#### 3.2 Release Build (Optimized)

```bash
cargo build --release
```

The output binary is at:
```
target/x86_64-unknown-none/release/lian-kernel
```

Release profile uses LTO (`lto = true`), single codegen unit, and `opt-level = 3` for maximum optimization.

#### 3.3 Generate Documentation

```bash
cargo doc --target x86_64-unknown-none --no-deps --open
```

This generates Rust API documentation in `target/x86_64-unknown-none/doc/lian/`.

#### 3.4 Clean Build Artifacts

```bash
cargo clean
```

Or with the Makefile:
```bash
make clean
```

---

### 4. Run in QEMU

Lian supports two boot methods: **direct kernel boot** (via PVH, no bootloader needed) and **ISO boot** (via GRUB Multiboot2). Direct boot is the recommended method.

#### 4.1 Direct Boot (Recommended)

This uses QEMU's `-kernel` flag to load the ELF binary directly. The kernel's PVH header tells QEMU how to boot it. No ISO or bootloader is needed.

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio
```

**With VGA display:**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -vga std
```

**Headless (serial output only, no GUI):**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none
```

> The `-m 256M` flag gives QEMU 256 MiB of RAM. The kernel's identity mapping covers the first 1 GiB, so any value up to 1 GiB works. Lower values (e.g., 128M) also work for v0.1.0.

#### 4.2 ISO Boot (Requires GRUB/xorriso)

First, build the ISO:
```bash
make iso
```

Then boot:
```bash
qemu-system-x86_64 \
  -drive format=raw,file=target/lian.iso \
  -m 256M \
  -serial stdio \
  -no-reboot \
  -no-shutdown
```

#### 4.3 Using the Makefile

The simplest way to build and run:

```bash
# Direct boot (no ISO needed)
make run-direct

# Build ISO and boot from it
make iso
make run
```

---

### 5. Expected Output

#### Serial Output

When the kernel boots successfully, the serial output (`-serial stdio`) shows:

```
Lian kernel: boot OK
```

#### VGA Output

If VGA is enabled (`-vga std`), the screen displays:

```
==================================================
 Lian Kernel v0.1.0
 Architecture: x86_64 (PVH boot, Long Mode)
==================================================
```

#### After Boot

After printing the welcome banner, the kernel enters an idle `hlt` loop. No further output is produced in v0.1.0. This is expected — the scheduler and interrupt handling are implemented but not yet wired into the boot sequence.

To exit QEMU, press `Ctrl+A` then `X` (in the QEMU monitor), or close the QEMU window.

---

### 6. Make Targets Reference

| Target | Command | Description |
|--------|---------|-------------|
| Build (debug) | `make build` | Compile the kernel in debug mode |
| Build (release) | `make release` | Compile with full optimizations |
| Build ISO | `make iso` | Create a bootable ISO image (requires xorriso/grub) |
| Run (ISO) | `make run` | Build ISO and boot in QEMU |
| Run (direct) | `make run-direct` | Boot kernel directly in QEMU (no ISO) |
| Test | `make test` | Run kernel tests in QEMU headless |
| Docs | `make doc` | Generate Rust documentation |
| Clean | `make clean` | Remove all build artifacts |
| GDB mode | `make run-gdb` | Start QEMU with GDB stub on port 1234 |
| Help | `make help` | List all available targets |

---

### 7. Debugging

#### 7.1 Serial Output

The primary debugging channel is the 16550 UART on COM1 (I/O port 0x3F8). All kernel messages are output here.

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none
```

Inside the kernel, use the serial macros:

```rust
serial_println!("debug: value = {}", x);
```

#### 7.2 QEMU Interrupt Log

To detect CPU exceptions (triple faults, page faults, GPFs, etc.):

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none \
  -d int \
  -D target/qemu_int.log
```

After the kernel runs for a few seconds, stop QEMU and check the log:

```bash
grep check_exception target/qemu_int.log
```

If the log is empty or contains no `check_exception` entries, the kernel is running without faults. If you see entries like `check_exception old: 0x... new: 0x...`, it indicates a triple fault or exception cascade.

#### 7.3 GDB Source-Level Debugging

**Step 1:** Start QEMU with the GDB stub (`-s` = listen on port 1234, `-S` = freeze CPU at start):

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -s -S
```

**Step 2:** In another terminal, connect GDB:

```bash
gdb target/x86_64-unknown-none/debug/lian-kernel
(gdb) target remote localhost:1234
(gdb) break kmain
(gdb) continue
(gdb) step
(gdb) info registers
```

Or with the Makefile:
```bash
make run-gdb    # Starts QEMU in GDB mode (background)
# Then in another terminal:
make gdb        # Prints the GDB connect command
```

#### 7.4 VGA Screenshot (QMP)

For headless verification of VGA output, use QEMU's QMP (QEMU Machine Protocol) to take a screenshot:

```bash
# Start QEMU with QMP on port 55555
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial null \
  -display none \
  -vga std \
  -qmp tcp:127.0.0.1:55555,server,nowait

# In another terminal, send QMP commands
echo '{"execute":"qmp_capabilities"}{"execute":"screendump","arguments":{"filename":"D:/XM/Lian/screenshot.ppm"}}' \
  | nc 127.0.0.1 55555
```

> **Note:** On Windows, the screenshot filename must use an absolute path with forward slashes (e.g., `D:/XM/Lian/screenshot.ppm`). The `/tmp/` path is not available on Windows.

#### 7.5 Panic Output

When the kernel panics, the panic handler outputs to both serial and VGA:

```
=== LIAN KERNEL PANIC ===
  at src/main.rs:295:5
  message: <panic message>
  halting CPU.
```

After printing the panic message, the kernel halts the CPU (`hlt` in an infinite loop). QEMU will appear frozen — exit with `Ctrl+A, X` or close the window.

---

### 8. Troubleshooting

#### "error: linking with `cc` failed"

The linker script (`linker.ld`) must be found. Ensure you are building from the project root directory where `linker.ld` resides:

```bash
cd /path/to/Lian
cargo build
```

The `.cargo/config.toml` passes `-Tlinker.ld` as a linker argument. If it's a relative path, it must resolve from the project root.

#### "error[E0463]: found a newer version of the Rust compiler"

The project uses Rust 2024 edition. Update your toolchain:

```bash
rustup update
```

#### QEMU: "Could not configure for multicast"

This is a known QEMU networking warning on some platforms. It does not affect kernel boot. Add `-nic none` to suppress it:

```bash
qemu-system-x86_64 -kernel ... -m 256M -serial stdio -nic none
```

#### QEMU exits immediately with no output

Check the following:

1. **Correct binary path:** Ensure you're using `target/x86_64-unknown-none/debug/lian-kernel` (not `target/debug/lian-kernel`).
2. **Debug build:** The release build has a different path (`target/x86_64-unknown-none/release/lian-kernel`).
3. **Serial routing:** Use `-serial stdio` to route COM1 to your terminal. Without this flag, serial output goes nowhere.

#### Triple fault / QEMU reboots repeatedly

If QEMU reboots continuously, the kernel is triple-faulting. Use the interrupt log to diagnose:

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none \
  -d int \
  -D target/qemu_int.log
```

Look for `check_exception` entries in `target/qemu_int.log`. A `check_exception old: 0xe new: 0xe` means a page fault occurred during the page fault handler — typically a stack or page table corruption.

#### `make iso` fails: "grub-mkrescue/xorriso not found"

Install the required tools:

```bash
# Ubuntu/Debian
sudo apt install xorriso grub-pc-bin

# macOS
brew install xorriso
```

Alternatively, use direct boot (`make run-direct`) which doesn't require an ISO.

#### VGA shows no output but serial works

Ensure you're passing `-vga std` (or omit `-display none`). The default QEMU display is `none` on headless systems. To see VGA:

```bash
qemu-system-x86_64 -kernel ... -m 256M -serial stdio -vga std
```

#### Code model errors during compilation

If you see `relocation truncated to fit: R_X86_64_...` errors, the `.cargo/config.toml` should already have `code-model=small`. Do **not** change it to `large` — the 32-bit boot stub uses 32-bit relocations that are incompatible with the large code model.

---

### 9. Platform-Specific Notes

#### 9.1 Windows

- **QEMU path:** Add QEMU to your `PATH` or use the full path. The winget installation typically places it in `C:\Program Files\qemu\`.
- **Cygwin/MSYS2/Git Bash:** The `make` command works, but QEMU's `-serial stdio` output may be interleaved. Use `-serial file:output.txt` if output is garbled.
- **Cargo in Git Bash:** If `cargo` is not found in Git Bash, add it to `PATH`:
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"
  ```
- **VGA screenshot:** Use Windows absolute paths with forward slashes (e.g., `D:/XM/Lian/screenshot.ppm`). The `/tmp/` path does not exist on Windows.
- **ISO creation:** Not supported natively on Windows. Use direct boot (`-kernel`) instead.

#### 9.2 Linux

- **Permissions:** QEMU may need `/dev/kvm` for hardware acceleration (not required for v0.1.0):
  ```bash
  sudo usermod -aG kvm $USER
  ```
- **Serial output:** Use `-serial stdio` or `-serial file:output.txt`.
- **ISO creation:** Fully supported with `xorriso` and `grub-pc-bin`.

#### 9.3 macOS

- **QEMU:** Install via Homebrew (`brew install qemu`). The binary is `qemu-system-x86_64`.
- **Apple Silicon (M1/M2):** QEMU will emulate x86_64 via TCG (software emulation). Performance is lower but functionality is identical.
- **GDB:** The default `gdb` from Homebrew may need `codesign` to work. Alternatively, use `lldb`:
  ```bash
  lldb target/x86_64-unknown-none/debug/lian-kernel
  (lldb) gdb-remote localhost:1234
  ```

---

*For architecture and development details, see [DEVELOPMENT.md](DEVELOPMENT.md).*

---

## 中文

### 目录

1. [前置条件](#1-前置条件)
2. [安装工具链](#2-安装工具链)
3. [构建内核](#3-构建内核)
4. [在 QEMU 中运行](#4-在-qemu-中运行)
5. [预期输出](#5-预期输出)
6. [Make 目标参考](#6-make-目标参考)
7. [调试](#7-调试)
8. [故障排除](#8-故障排除)
9. [平台特定说明](#9-平台特定说明)

---

### 1. 前置条件

| 工具 | 必需 | 最低版本 | 用途 |
|------|------|----------|------|
| **Rust** | 是 | 1.98+（stable, 2024 edition） | 编译内核 |
| **Rust target `x86_64-unknown-none`** | 是 | — | 裸机 target triple |
| **QEMU** | 推荐 | 7.0+（`qemu-system-x86_64`） | 启动和测试内核 |
| **xorriso** 或 **grub-mkrescue** | 可选 | — | 构建可引导 ISO 镜像 |
| **GDB** | 可选 | 10+ | 源码级调试 |
| **make** | 可选 | 4.0+ | 使用 Makefile 目标 |

> 如果只想直接构建和运行内核（不使用 Makefile），只需 Rust + QEMU。

---

### 2. 安装工具链

#### 2.1 安装 Rust

通过 [rustup](https://rustup.rs) 安装：

**Linux / macOS / WSL：**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

**Windows：**
从 [https://rustup.rs](https://rustup.rs) 下载并运行 `rustup-init.exe`，或通过 winget 安装：
```bash
winget install Rustlang.Rustup
```

验证安装：
```bash
rustc --version
cargo --version
```

> **注意：** 项目需要 Rust 2024 edition，Rust 1.98 及以上版本支持。如果版本较低，运行 `rustup update`。

#### 2.2 添加裸机 Target

```bash
rustup target add x86_64-unknown-none
```

此 target 是"裸机"triple — 没有操作系统、没有标准库、没有运行时。`.cargo/config.toml` 已预配置为自动使用它。

#### 2.3 安装 QEMU

**Linux (Debian/Ubuntu)：**
```bash
sudo apt install qemu-system-x86
```

**Linux (Fedora)：**
```bash
sudo dnf install qemu-system-x86-core
```

**macOS (Homebrew)：**
```bash
brew install qemu
```

**Windows (winget)：**
```bash
winget install qemu.qemu
```

验证：
```bash
qemu-system-x86_64 --version
```

#### 2.4（可选）安装 ISO 构建工具

仅当需要构建可引导 ISO 镜像（`make iso`）时才需要：

**Linux (Debian/Ubuntu)：**
```bash
sudo apt install xorriso grub-pc-bin
```

**macOS：**
```bash
brew install xorriso
```

**Windows：** Windows 原生不支持 ISO 创建。请改用直接 `-kernel` 引导方式（见[第 4.1 节](#41-直接引导推荐)）。

#### 2.5（可选）安装 GDB

**Linux：** `sudo apt install gdb`  
**macOS：** `brew install gdb`  
**Windows：** 通过 MinGW 安装或使用 Rust 工具链自带的 `rust-gdb`。

---

### 3. 构建内核

#### 3.1 Debug 构建

```bash
cargo build
```

或显式指定：
```bash
cargo build --target x86_64-unknown-none
```

输出二进制文件位于：
```
target/x86_64-unknown-none/debug/lian-kernel
```

> `.cargo/config.toml` 自动设置 target、链接脚本（`linker.ld`）和 code model。无需传递额外标志。

#### 3.2 Release 构建（优化）

```bash
cargo build --release
```

输出二进制文件位于：
```
target/x86_64-unknown-none/release/lian-kernel
```

Release profile 使用 LTO（`lto = true`）、单 codegen unit 和 `opt-level = 3` 以获得最大优化。

#### 3.3 生成文档

```bash
cargo doc --target x86_64-unknown-none --no-deps --open
```

这会在 `target/x86_64-unknown-none/doc/lian/` 生成 Rust API 文档。

#### 3.4 清理构建产物

```bash
cargo clean
```

或使用 Makefile：
```bash
make clean
```

---

### 4. 在 QEMU 中运行

Lian 支持两种引导方式：**直接内核引导**（通过 PVH，无需引导器）和 **ISO 引导**（通过 GRUB Multiboot2）。直接引导是推荐方式。

#### 4.1 直接引导（推荐）

使用 QEMU 的 `-kernel` 标志直接加载 ELF 二进制文件。内核的 PVH 头部告诉 QEMU 如何引导。无需 ISO 或引导器。

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio
```

**带 VGA 显示：**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -vga std
```

**无头模式（仅串口输出，无 GUI）：**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none
```

> `-m 256M` 标志给 QEMU 256 MiB 内存。内核的身份映射覆盖前 1 GiB，因此不超过 1 GiB 的任何值都可以。v0.1.0 也支持更低的值（如 128M）。

#### 4.2 ISO 引导（需要 GRUB/xorriso）

首先，构建 ISO：
```bash
make iso
```

然后引导：
```bash
qemu-system-x86_64 \
  -drive format=raw,file=target/lian.iso \
  -m 256M \
  -serial stdio \
  -no-reboot \
  -no-shutdown
```

#### 4.3 使用 Makefile

最简单的构建和运行方式：

```bash
# 直接引导（无需 ISO）
make run-direct

# 构建 ISO 并从中引导
make iso
make run
```

---

### 5. 预期输出

#### 串口输出

内核成功引导后，串口输出（`-serial stdio`）显示：

```
Lian kernel: boot OK
```

#### VGA 输出

如果启用了 VGA（`-vga std`），屏幕显示：

```
==================================================
 Lian Kernel v0.1.0
 Architecture: x86_64 (PVH boot, Long Mode)
==================================================
```

#### 引导后

打印欢迎横幅后，内核进入空闲 `hlt` 循环。v0.1.0 不会产生更多输出。这是预期行为 — 调度器和中断处理已实现但尚未接入引导序列。

退出 QEMU：按 `Ctrl+A` 然后 `X`（在 QEMU monitor 中），或关闭 QEMU 窗口。

---

### 6. Make 目标参考

| 目标 | 命令 | 描述 |
|------|------|------|
| 构建 (debug) | `make build` | 以 debug 模式编译内核 |
| 构建 (release) | `make release` | 以完整优化编译 |
| 构建 ISO | `make iso` | 创建可引导 ISO 镜像（需要 xorriso/grub） |
| 运行 (ISO) | `make run` | 构建 ISO 并在 QEMU 中引导 |
| 运行 (直接) | `make run-direct` | 直接在 QEMU 中引导内核（无需 ISO） |
| 测试 | `make test` | 在 QEMU 无头模式下运行内核测试 |
| 文档 | `make doc` | 生成 Rust 文档 |
| 清理 | `make clean` | 清除所有构建产物 |
| GDB 模式 | `make run-gdb` | 在端口 1234 启动带 GDB stub 的 QEMU |
| 帮助 | `make help` | 列出所有可用目标 |

---

### 7. 调试

#### 7.1 串口输出

主要调试通道是 COM1（I/O 端口 0x3F8）上的 16550 UART。所有内核消息都输出到这里。

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none
```

在内核中使用串口宏：

```rust
serial_println!("debug: value = {}", x);
```

#### 7.2 QEMU 中断日志

检测 CPU 异常（三重故障、页错误、GPF 等）：

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none \
  -d int \
  -D target/qemu_int.log
```

内核运行几秒后，停止 QEMU 并检查日志：

```bash
grep check_exception target/qemu_int.log
```

如果日志为空或不含 `check_exception` 条目，表示内核运行无故障。如果看到 `check_exception old: 0x... new: 0x...` 条目，表示三重故障或异常级联。

#### 7.3 GDB 源码级调试

**步骤 1：** 启动带 GDB stub 的 QEMU（`-s` = 监听端口 1234，`-S` = 启动时冻结 CPU）：

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -s -S
```

**步骤 2：** 在另一个终端连接 GDB：

```bash
gdb target/x86_64-unknown-none/debug/lian-kernel
(gdb) target remote localhost:1234
(gdb) break kmain
(gdb) continue
(gdb) step
(gdb) info registers
```

或使用 Makefile：
```bash
make run-gdb    # 在 GDB 模式下启动 QEMU（后台）
# 然后在另一个终端：
make gdb        # 打印 GDB 连接命令
```

#### 7.4 VGA 截图（QMP）

用于无头模式下验证 VGA 输出，使用 QEMU 的 QMP（QEMU Machine Protocol）截图：

```bash
# 启动带 QMP 的 QEMU，端口 55555
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial null \
  -display none \
  -vga std \
  -qmp tcp:127.0.0.1:55555,server,nowait

# 在另一个终端发送 QMP 命令
echo '{"execute":"qmp_capabilities"}{"execute":"screendump","arguments":{"filename":"D:/XM/Lian/screenshot.ppm"}}' \
  | nc 127.0.0.1 55555
```

> **注意：** 在 Windows 上，截图文件名必须使用正斜杠的绝对路径（如 `D:/XM/Lian/screenshot.ppm`）。Windows 上没有 `/tmp/` 路径。

#### 7.5 Panic 输出

内核 panic 时，panic 处理器同时输出到串口和 VGA：

```
=== LIAN KERNEL PANIC ===
  at src/main.rs:295:5
  message: <panic 消息>
  halting CPU.
```

打印 panic 消息后，内核停机 CPU（`hlt` 无限循环）。QEMU 会看起来冻结 — 按 `Ctrl+A, X` 退出或关闭窗口。

---

### 8. 故障排除

#### "error: linking with `cc` failed"

必须找到链接脚本（`linker.ld`）。确保从 `linker.ld` 所在的项目根目录构建：

```bash
cd /path/to/Lian
cargo build
```

`.cargo/config.toml` 传递 `-Tlinker.ld` 作为链接器参数。如果是相对路径，必须从项目根目录解析。

#### "error[E0463]: found a newer version of the Rust compiler"

项目使用 Rust 2024 edition。更新工具链：

```bash
rustup update
```

#### QEMU："Could not configure for multicast"

这是某些平台上已知的 QEMU 网络警告。不影响内核引导。添加 `-nic none` 抑制：

```bash
qemu-system-x86_64 -kernel ... -m 256M -serial stdio -nic none
```

#### QEMU 立即退出且无输出

检查以下事项：

1. **正确的二进制路径：** 确保使用 `target/x86_64-unknown-none/debug/lian-kernel`（不是 `target/debug/lian-kernel`）。
2. **Debug 构建：** Release 构建路径不同（`target/x86_64-unknown-none/release/lian-kernel`）。
3. **串口路由：** 使用 `-serial stdio` 将 COM1 路由到终端。没有此标志，串口输出无处可去。

#### 三重故障 / QEMU 反复重启

如果 QEMU 持续重启，内核正在三重故障。使用中断日志诊断：

```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none \
  -d int \
  -D target/qemu_int.log
```

在 `target/qemu_int.log` 中查找 `check_exception` 条目。`check_exception old: 0xe new: 0xe` 表示页错误处理期间又发生了页错误 — 通常是栈或页表损坏。

#### `make iso` 失败："grub-mkrescue/xorriso not found"

安装所需工具：

```bash
# Ubuntu/Debian
sudo apt install xorriso grub-pc-bin

# macOS
brew install xorriso
```

或者使用直接引导（`make run-direct`），不需要 ISO。

#### VGA 无输出但串口正常

确保传递了 `-vga std`（或省略 `-display none`）。无头系统上 QEMU 默认显示为 `none`。要查看 VGA：

```bash
qemu-system-x86_64 -kernel ... -m 256M -serial stdio -vga std
```

#### 编译时 code model 错误

如果看到 `relocation truncated to fit: R_X86_64_...` 错误，`.cargo/config.toml` 应该已设置 `code-model=small`。**不要**改为 `large` — 32 位引导存根使用 32 位重定位，与 large code model 不兼容。

---

### 9. 平台特定说明

#### 9.1 Windows

- **QEMU 路径：** 将 QEMU 添加到 `PATH` 或使用完整路径。winget 安装通常放在 `C:\Program Files\qemu\`。
- **Cygwin/MSYS2/Git Bash：** `make` 命令可用，但 QEMU 的 `-serial stdio` 输出可能交错。如果输出混乱，使用 `-serial file:output.txt`。
- **Git Bash 中的 Cargo：** 如果在 Git Bash 中找不到 `cargo`，添加到 `PATH`：
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"
  ```
- **VGA 截图：** 使用 Windows 绝对路径加正斜杠（如 `D:/XM/Lian/screenshot.ppm`）。Windows 上没有 `/tmp/` 路径。
- **ISO 创建：** Windows 原生不支持。改用直接引导（`-kernel`）。

#### 9.2 Linux

- **权限：** QEMU 可能需要 `/dev/kvm` 进行硬件加速（v0.1.0 不需要）：
  ```bash
  sudo usermod -aG kvm $USER
  ```
- **串口输出：** 使用 `-serial stdio` 或 `-serial file:output.txt`。
- **ISO 创建：** 使用 `xorriso` 和 `grub-pc-bin` 完全支持。

#### 9.3 macOS

- **QEMU：** 通过 Homebrew 安装（`brew install qemu`）。二进制为 `qemu-system-x86_64`。
- **Apple Silicon (M1/M2)：** QEMU 通过 TCG（软件模拟）模拟 x86_64。性能较低但功能完全相同。
- **GDB：** Homebrew 的默认 `gdb` 可能需要 `codesign` 才能工作。或者使用 `lldb`：
  ```bash
  lldb target/x86_64-unknown-none/debug/lian-kernel
  (lldb) gdb-remote localhost:1234
  ```

---

*架构和开发详情请参见 [DEVELOPMENT.md](DEVELOPMENT.md)。*
