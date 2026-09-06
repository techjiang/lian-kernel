# Contributing to Lian / 贡献指南

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-C0778E.svg)](LICENSE-MIT)[![Rust Version](https://img.shields.io/badge/rust-1.98%2B%20(2024%20edition)-C0778E.svg)](https://www.rust-lang.org)[![Platform](https://img.shields.io/badge/platform-x86__64%20bare%20metal-C0778E.svg)](#)[![Dependencies](https://img.shields.io/badge/dependencies-0%20(zero%20crates.io)-C0778E.svg)](#)[![Version](https://img.shields.io/badge/version-0.1.0-C0778E.svg)](#)[![author](https://img.shields.io/badge/作者-科技酱-C0778E.svg)](https://docs.asoe.cn)

Thank you for your interest in contributing to the Lian kernel! This document outlines the process for submitting changes.

感谢你对 Lian 内核项目的关注！本文档概述了提交变更的流程。

> **Author / 作者:** [科技酱](https://docs.asoe.cn)  
> **Project / 项目:** Lian Kernel

**[English](#english) | [中文](#中文)**

---

## English

### Getting Started

1. **Fork** the repository on GitHub.
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/Lian.git
   cd Lian
   ```
3. **Install the toolchain** (see [docs/USAGE.md](docs/USAGE.md) for details):
   ```bash
   rustup target add x86_64-unknown-none
   ```
4. **Verify the build**:
   ```bash
   cargo build
   ```
5. **Verify it boots**:
   ```bash
   qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio -display none
   ```
   You should see `Lian kernel: boot OK`.

---

### Development Workflow

#### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bugfix-name
```

#### 2. Make Your Changes

Follow the [code conventions](docs/DEVELOPMENT.md#10-code-conventions):

- Use **Rust 2024 edition** conventions.
- All `unsafe` blocks must have `// SAFETY:` comments explaining why the operation is safe.
- No external crate dependencies — everything must be implemented from scratch.
- Use `snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.

#### 3. Test Your Changes

**Build check:**
```bash
cargo build
```

**Boot test:**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none
```

**Interrupt log (check for faults):**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none \
  -d int \
  -D target/qemu_int.log

# Verify no exceptions
grep check_exception target/qemu_int.log
# Empty output = no faults
```

#### 4. Commit Your Changes

Use clear, descriptive commit messages:

```
feat: add APIC timer support for x86_64

- Implement local APIC initialization in arch/x86_64/apic.rs
- Replace 8259 PIC EOI with APIC EOI when APIC is active
- Keep 8259 PIC as fallback for non-APIC systems

Closes #42
```

**Commit message format:**
- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation only
- `refactor:` — Code restructuring (no behavior change)
- `test:` — Adding tests
- `chore:` — Build/config/tooling

#### 5. Push and Create a Pull Request

```bash
git push origin feature/your-feature-name
```

Open a Pull Request on GitHub with:
- A clear title describing the change
- A description of what was changed and why
- Reference to any related issues (`Closes #123`, `Refs #456`)
- Confirmation that the build passes and the kernel boots

---

### Code Review Criteria

All PRs are reviewed against the following criteria:

| Criterion | Requirement |
|-----------|-------------|
| **Builds cleanly** | `cargo build` succeeds with no errors |
| **Boots successfully** | Kernel outputs `boot OK` in QEMU |
| **No CPU exceptions** | QEMU interrupt log shows no `check_exception` |
| **No new warnings** | No new compiler warnings introduced |
| **Zero external deps** | No new crates.io dependencies added |
| **Unsafe documented** | All `unsafe` blocks have `// SAFETY:` comments |
| **Code style** | Follows conventions in DEVELOPMENT.md |
| **Tests included** | New functionality is testable in QEMU |

---

### Reporting Bugs

Use [GitHub Issues](https://github.com/techjiang/lian-kernel/issues) to report bugs. Include:

1. **Environment:** Rust version (`rustc --version`), QEMU version, OS
2. **Steps to reproduce:** Exact commands to trigger the issue
3. **Expected vs actual behavior:** What you expected vs what happened
4. **Serial output:** The full serial output from QEMU
5. **Interrupt log:** If applicable, attach `qemu_int.log` contents
6. **GDB backtrace:** If the kernel panics, provide the panic output

---

### Suggested Areas for Contribution

- **APIC/IOAPIC support** — Replace legacy 8259 PIC with APIC
- **ACPI parsing** — Parse ACPI tables for hardware enumeration
- **User-space (ring 3)** — Implement user-mode transitions and TSS IST stacks
- **Real filesystem** — Implement ext2 or FAT32 read support
- **ARM64 port** — Implement HAL traits for AArch64
- **Networking** — E1000 or RTL8139 driver with a network stack
- **Testing** — Automated boot tests in CI (GitHub Actions)

---

### Questions?

- Open an [issue](https://github.com/techjiang/lian-kernel/issues) for bugs and feature requests.
- Visit [https://docs.asoe.cn](https://docs.asoe.cn) for documentation.

Thank you for contributing to Lian!

---

## 中文

### 快速上手

1. 在 GitHub 上 **Fork** 仓库。
2. 将你的 fork **克隆**到本地：
   ```bash
   git clone https://github.com/<your-username>/Lian.git
   cd Lian
   ```
3. **安装工具链**（详见 [docs/USAGE.md](docs/USAGE.md)）：
   ```bash
   rustup target add x86_64-unknown-none
   ```
4. **验证构建**：
   ```bash
   cargo build
   ```
5. **验证启动**：
   ```bash
   qemu-system-x86_64 -kernel target/x86_64-unknown-none/debug/lian-kernel -m 256M -serial stdio -display none
   ```
   应看到 `Lian kernel: boot OK`。

---

### 开发流程

#### 1. 创建分支

```bash
git checkout -b feature/your-feature-name
# 或
git checkout -b fix/your-bugfix-name
```

#### 2. 进行修改

遵循[代码规范](docs/DEVELOPMENT.md#10-code-conventions)：

- 使用 **Rust 2024 edition** 惯例。
- 所有 `unsafe` 块必须有 `// SAFETY:` 注释说明操作为何安全。
- 禁止外部 crate 依赖 — 一切必须从零实现。
- 函数/变量用 `snake_case`，类型用 `PascalCase`，常量用 `SCREAMING_SNAKE_CASE`。

#### 3. 测试修改

**构建检查：**
```bash
cargo build
```

**启动测试：**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none
```

**中断日志（检查故障）：**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/lian-kernel \
  -m 256M \
  -serial stdio \
  -display none \
  -d int \
  -D target/qemu_int.log

# 验证无异常
grep check_exception target/qemu_int.log
# 空输出 = 无故障
```

#### 4. 提交修改

使用清晰、描述性的提交信息：

```
feat: add APIC timer support for x86_64

- Implement local APIC initialization in arch/x86_64/apic.rs
- Replace 8259 PIC EOI with APIC EOI when APIC is active
- Keep 8259 PIC as fallback for non-APIC systems

Closes #42
```

**提交信息格式：**
- `feat:` — 新功能
- `fix:` — Bug 修复
- `docs:` — 仅文档
- `refactor:` — 代码重构（无行为变更）
- `test:` — 添加测试
- `chore:` — 构建/配置/工具

#### 5. 推送并创建 Pull Request

```bash
git push origin feature/your-feature-name
```

在 GitHub 上创建 Pull Request，包含：
- 清晰的标题描述变更
- 描述改了什么以及为什么
- 引用相关 issue（`Closes #123`、`Refs #456`）
- 确认构建通过且内核可启动

---

### 代码审查标准

所有 PR 按以下标准审查：

| 标准 | 要求 |
|------|------|
| **构建成功** | `cargo build` 无错误 |
| **启动成功** | 内核在 QEMU 中输出 `boot OK` |
| **无 CPU 异常** | QEMU 中断日志无 `check_exception` |
| **无新警告** | 不引入新的编译器警告 |
| **零外部依赖** | 不添加新的 crates.io 依赖 |
| **Unsafe 有文档** | 所有 `unsafe` 块有 `// SAFETY:` 注释 |
| **代码风格** | 遵循 DEVELOPMENT.md 中的规范 |
| **包含测试** | 新功能可在 QEMU 中测试 |

---

### 报告 Bug

使用 [GitHub Issues](https://github.com/techjiang/lian-kernel/issues) 报告 Bug。请包含：

1. **环境：** Rust 版本（`rustc --version`）、QEMU 版本、操作系统
2. **复现步骤：** 触发问题的确切命令
3. **预期与实际行为：** 预期什么 vs 实际发生了什么
4. **串口输出：** QEMU 的完整串口输出
5. **中断日志：** 如适用，附上 `qemu_int.log` 内容
6. **GDB 回溯：** 如内核 panic，提供 panic 输出

---

### 建议的贡献方向

- **APIC/IOAPIC 支持** — 用 APIC 替换传统 8259 PIC
- **ACPI 解析** — 解析 ACPI 表进行硬件枚举
- **用户态 (ring 3)** — 实现用户态转换和 TSS IST 栈
- **真实文件系统** — 实现 ext2 或 FAT32 读取支持
- **ARM64 移植** — 为 AArch64 实现 HAL trait
- **网络** — E1000 或 RTL8139 驱动加网络栈
- **测试** — CI 中的自动化启动测试（GitHub Actions）

---

### 有问题？

- 在 [issue](https://github.com/techjiang/lian-kernel/issues) 中提交 bug 和功能请求。
- 访问 [https://docs.asoe.cn](https://docs.asoe.cn) 查看文档。

感谢你对 Lian 的贡献！
