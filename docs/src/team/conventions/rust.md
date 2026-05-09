# Rust 编码规范

> 目标：写出让三个月后的自己和同事都能一眼看懂、安全、可测试的 Rust 代码。
> 每条规则都有 **Why** 和 **Example**，而非罗列条款。

---

## 1. 总则

### 1.1 代码是写给人看的

编译器不关心你的变量名叫 `x` 还是 `user_home_dir`，但三个月后的你会关心。

### 1.2 利用编译器

Rust 编译器是你最好的 reviewer。凡是能在编译期解决的问题，不要留到运行时。

- **优先使用类型系统表达不变量**，而非运行时断言。能用 `enum` 表达的状态机，就不要用 `bool` + `if`。
- **`clippy::pedantic` 不是噪声**。每条 lint 背后都是一个真实的 bug 模式。

### 1.3 一致性优于个人偏好

如果你在改一个已有文件，请保持和周围代码一致的风格。如果你发现整个项目都不一致，提一个单独的 cleanup commit——不要把风格修正和功能改动混在同一个 commit 里。

---

## 2. 命名

| 元素 | 约定 | 示例 |
| --- | --- | --- |
| 模块/文件名 | `snake_case` | `pipe_executor.rs`, `job_control.rs` |
| 类型/特质/枚举变体 | `PascalCase` | `Pipeline`, `ParseResult<T>`, `RedirectionError` |
| 函数/方法 | `snake_case` | `parse_command`, `redirect_stdin` |
| 常量/静态变量 | `SCREAMING_SNAKE_CASE` | `MAX_PIPE_DEPTH`, `DEFAULT_PROMPT` |
| 局部变量/参数 | `snake_case` | `cmd_line`, `exit_code` |
| 特质方法前缀 | `as_`/`to_`/`into_` 遵循 C-CONV | `as_bytes()`, `to_owned()`, `into_iter()` |
| 构造器 | `new` (无参) 或 `with_*` (带参) | `Shell::new()`, `Command::with_args(vec![])` |

**Why**: 命名是代码可读性的第一道防线。当读者看到 `PascalCase` 就知道这是一个类型，看到 `snake_case` 就知道是一个值/函数。Rust 编译器也会对不符合约定的命名发出 warning。

### 2.1 命名的精确度

- **选词要具体**：`bytes_read` 优于 `n`，`exit_code` 优于 `code`，`child_pid` 优于 `pid`。
- **布尔值用 `is_`/`has_`/`should_` 前缀**：`is_background`, `has_pipe`, `should_exit`。
- **避免泛化词**：`data`, `info`, `item`, `value`, `result` 在多数场景下是噪音——优先用更具体的名称。

```rust
// 差
let data = parse(s);
let result = execute(data);

// 好
let pipeline = parse(input_line);
let exit_status = execute(pipeline);
```

### 2.2 缩写规则

- **全大写缩写**（如 `PID`, `Fd`）在 PascalCase 中保持原样：`PidfdReader`；在 snake_case 中全小写：`pidfd_reader`。
- 除了极其通用的缩写（`io`, `fs`, `os`），不要在标识符中凭空造缩写。

---

## 3. 格式化

### 3.1 自动化工具为唯一标准

- **使用 `cargo fmt` 处理所有格式问题**。不要手动调整缩进、换行、空格。
- 在项目根目录放置 `rustfmt.toml`，将所有风格讨论收敛到这一个文件。
- 每个 commit 前运行 `cargo fmt --check`。

```toml
# rustfmt.toml（推荐配置）
edition = "2024"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Module"
group_imports = "StdExternalCrate"
reorder_impl_items = true
format_strings = true
```

**Why**: 格式问题不值得 code review 时讨论。把决策权交给 `rustfmt`，把认知资源留给逻辑和架构。

### 3.2 行宽与换行

- `max_width = 100`。现代显示器足够宽，`rustfmt` 会在必要时自动换行。
- 如果某个表达式 `rustfmt` 换行后仍然难以阅读，那就说明它应该被提取为一个有名字的中间变量。

---

## 4. 代码组织

### 4.1 模块结构

- **每个文件只暴露一个核心类型或一个核心功能**。如果一个文件里塞了三个 `pub struct`，拆成三个文件。
- 模块树应与功能分解对齐，而非类型分类：

```text
src/
├── main.rs              # 入口，只做参数解析和顶层调度
├── parser.rs            # 词法/语法分析 → AST
├── ast.rs               # AST 节点定义（纯数据，无逻辑）
├── executor.rs          # 命令执行调度
├── builtins.rs           # cd/exit/export 等内置命令
├── redirection.rs        # 输入输出重定向
├── pipe.rs               # 管道逻辑
├── jobs.rs               # 后台作业管理
├── signal.rs             # 信号处理
├── error.rs              # 统一错误类型
└── syscall/              # 系统调用封装子模块
    ├── mod.rs
    ├── process.rs
    └── fd.rs
```

**Why**: 模块树即架构图。一个新人打开 `src/` 目录，应该能从文件名推断出系统的组成部分。

### 4.2 `mod.rs` vs 同名文件

```text
# 推荐：使用 mod.rs（保持目录可迁移性）
syscall/mod.rs
syscall/process.rs

# 也可：同名文件风格（Rust 2024 edition 后更主流）
syscall.rs
syscall/process.rs
```

**选择一个并全局保持一致**。本规范推荐 `mod.rs` 风格，因为删除/移动目录时不会留下 orphan 文件。

### 4.3 可见性

- **默认 `pub(crate)`，而非 `pub`**。除非你确实在写一个会被外部 crate 依赖的 library。
- `pub` 是 API 承诺。每次你写 `pub`，就问自己："如果这个签名要改，我会不会犹豫？"——如果会，它就不该是 `pub`。
- 不要为了测试把东西设为 `pub`。使用 `#[cfg(test)]` 模块直接访问 `pub(crate)` 项。

```rust
// 好：最小化公开面
pub(crate) struct Pipeline {
    commands: Vec<Command>,      // 私有字段
}

impl Pipeline {
    pub(crate) fn new(commands: Vec<Command>) -> Self { ... }
    pub(crate) fn execute(&self) -> Result<Vec<i32>> { ... }
}
```

### 4.4 `use` 语句

- **禁止 `use crate::some::module::*`**（`*` 引入）。它让读者无法追踪符号来源。
- 对标准库和外部 crate，使用完整路径引入；对 crate 内部，优先 `use crate::parser::Parser`。
- `use` 语句放在文件顶部，按 `std` → 外部 crate → `crate` 三段分组，组间空一行。

```rust
use std::collections::HashMap;
use std::path::PathBuf;

use nix::sys::signal::Signal;
use nix::unistd::Pid;

use crate::ast::{Command, Pipeline};
use crate::error::ShellError;
```

---

## 5. 类型系统

### 5.1 enum >> bool

如果你有两个状态，今天就一个 `bool`。如果明天可能变成三个，今天就写 `enum`。

```rust
// 差
fn execute(cmd: &Command, background: bool) -> Result<i32> { ... }

// 好
enum ExecutionMode {
    Foreground,
    Background,
}

fn execute(cmd: &Command, mode: ExecutionMode) -> Result<i32> { ... }
```

### 5.2 善用 newtype

具有语义区别的基本类型，应包装为 newtype 避免混淆：

```rust
// 差：两个 i32 容易传错顺序
fn wait_pid(pid: i32, timeout_ms: i32) -> Result<ExitStatus> { ... }

// 好：编译器帮你检查
#[derive(Debug, Clone, Copy)]
struct ProcessId(i32);

#[derive(Debug, Clone, Copy)]
struct TimeoutMs(u64);

fn wait_pid(pid: ProcessId, timeout: TimeoutMs) -> Result<ExitStatus> { ... }
```

### 5.3 不要滥用派生宏

**可以随意 derive 的**：`Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`。
**需要思考的**：`Copy`（只有轻量、语义上可复制的类型才加——`Pid` 是 `Copy`，`Vec<Command>` 不是）。
**永远不要随意 derive 的**：`Default`——如果"默认值"在你的领域中没有明确含义，不要加。

### 5.4 构造函数

- 每个 `struct` 应有一个明确的构造入口，通常是 `new()` 或 `with_xxx()`。
- 构造函数应 **构造** 对象，而非执行副作用（不读写文件、不 fork 进程）。

```rust
impl Shell {
    /// 构造一个新的 Shell 实例。
    ///
    /// 此方法不分配终端、不设置信号处理——只是创建数据结构。
    pub(crate) fn new() -> Self {
        Self {
            jobs: JobTable::new(),
            exit_code: 0,
        }
    }
}
```

---

## 6. 错误处理

### 6.1 两条铁律

1. **库代码**：永远不 panic。返回 `Result<T, E>` 或 `Option<T>`。
2. **应用程序**：只在"继续执行会导致数据损坏"或"启动阶段配置缺失"时允许 panic。

### 6.2 错误类型设计

使用 `thiserror` 定义领域错误，而非手写 `Display` + `Error` impl：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ShellError {
    #[error("command not found: {0}")]
    CommandNotFound(String),

    #[error("failed to redirect fd {fd} to {target}")]
    Redirection { fd: i32, target: String, #[source] source: io::Error },

    #[error("pipe creation failed")]
    Pipe(#[source] io::Error),

    #[error("fork failed")]
    Fork(#[source] nix::Error),

    #[error("signal error: {0}")]
    Signal(String),
}
```

**Why `thiserror`**：手写 `Display` + `Error` + `source` 的样板代码容易出错，且当错误变体增加时维护成本高。

### 6.3 错误传播

- 使用 `?` 传播错误。不要写 `match` + `return Err(...)`。
- 使用 `anyhow::Context` 为错误附加调用上下文（仅限应用层，库层用 `thiserror` 的 `#[source]`）。

```rust
use anyhow::Context;

fn run_script(path: &Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read script: {}", path.display()))?;
    // ...
}
```

### 6.4 `unwrap()` 和 `expect()` 使用标准

- **`unwrap()`**：禁止。它不携带任何上下文信息。
- **`expect("why this can't fail")`**：仅在"逻辑上不可能失败"的位置使用，且 `expect` 消息必须解释 **为什么** 不可能，而非描述 **如果失败是什么错误**。

```rust
// 差：消息只是描述了失败场景
let home = std::env::var("HOME").expect("HOME not set");

// 好：解释了为什么不可能失败
let home = std::env::var("HOME")
    .expect("POSIX requires HOME to be set for login shells");

// 更好：用 Result 传播
let home = std::env::var("HOME").context("HOME environment variable not set")?;
```

### 6.5 系统调用错误的特殊处理

因为是 OS lab，你可能直接面对 `libc` 返回值。规则：

- **不要吞掉 `errno`**。错误信息中必须包含 `errno` 的值和含义。
- 使用 `nix` crate 提供的类型安全封装，而非直接调用 `libc::fork()`。
- 对裸 `libc` 调用，立即将返回值转为 `Result`，不要延迟到下一行。

```rust
// 差：errno 可能被中间调用覆盖
let ret = unsafe { libc::close(fd) };
if ret < 0 {
    let msg = format!("close({fd}) failed"); // errno 可能已经不是原来的了
}

// 好：使用 nix 封装
nix::unistd::close(fd).context("close stdin in child process")?;

// 如果必须用 libc：立即保存 errno
let ret = unsafe { libc::close(fd) };
if ret < 0 {
    let err = io::Error::last_os_error();
    return Err(ShellError::Syscall { call: "close", fd, source: err });
}
```

---

## 7. 所有权与借用

### 7.1 永远不要为了"方便"使用 `clone()`

如果你写 `.clone()` 只是为了不跟 borrow checker 打交道，停下来想三秒：

1. 能否改为借用（`&T`）？
2. 能否移交所有权（move）？
3. 能否用 `Rc`/`Arc` 共享？
4. 如果以上都不行——clone 也可以，但加一行注释说明原因。

```rust
// 差：只看一眼代码不知道为什么要 clone
let cmd = commands[0].clone();
execute(cmd);

// 好：借用即可
execute(&commands[0]);

// 可接受：有明确语义需求
let cmd = commands[0].clone(); // Clone：需要把 cmd 移到后台线程
thread::spawn(move || execute(cmd));
```

### 7.2 优先使用引用而非智能指针

- `&T` > `Box<T>` > `Rc<T>` > `Arc<T>`。只在必要时提升。
- 如果函数不需要拥有数据的所有权，就接受 `&T` 或 `&str` 而非 `T` 或 `String`。

```rust
// 差：函数不需要所有权
fn parse_command(line: String) -> Command { ... }

// 好
fn parse_command(line: &str) -> Command { ... }
```

### 7.3 生命周期标注

- 大多数场景下，生命周期省略规则能处理。如果手动标注 `'a`，在函数签名上方加一行注释解释 `'a` 代表什么。
- 如果发现自己在同一个 `impl` 块里写了 3 个不同的生命周期参数，说明数据结构设计可能需要重新考虑。

---

## 8. 文档与注释

### 8.1 注释哲学

代码告诉你 **做了什么**，注释告诉你 **为什么这么做**。

以下情况需要注释：

- 非显而易见的算法或数据结构选择（"为什么用 BTreeMap 而非 HashMap"）
- 某段代码存在的原因（"解决某 bug 的 workaround"）
- `unsafe` 块的前置条件（见第 10 节）
- 公开 API 的文档

以下情况不需要注释：

- 代码本身已经清晰表达的（"increment counter" 不需要注释）
- 明显的 `// TODO` 注释——要么修掉，要么提到 Issue 追踪

### 8.2 Doc comment（`///` 和 `//!`）

规则：

1. **每个 `pub` 项都必须有 doc comment**（包括 `pub(crate)`）。
2. **首行是一个以句号结尾的单句摘要**。`rustdoc` 将其作为列表页面的描述。
3. **空行后写详细说明**。
4. **使用 Markdown 格式**。`` ` `` 包裹代码片段，```` ```rust ```` 包裹代码块。
5. **必须包含一个 `# Examples` 部分**，且示例应该可以通过 `cargo test` 编译和运行。

```rust
/// Parses an input line into an AST of shell commands.
///
/// This function tokenizes the input and constructs a [`Pipeline`] of
/// [`Command`] nodes representing pipes, redirections, and boolean
/// operators. It does **not** interpret quoted strings or perform
/// variable expansion—only syntax-level parsing.
///
/// # Errors
///
/// Returns [`ShellError::Parse`] if the input contains unmatched
/// quotes or malformed redirections.
///
/// # Examples
///
/// ```
/// use osh::parser::parse;
///
/// let pipeline = parse("ls -la | wc -l")?;
/// assert_eq!(pipeline.len(), 2);
/// # Ok::<(), osh::error::ShellError>(())
/// ```
pub(crate) fn parse(input: &str) -> Result<Pipeline, ShellError> {
    // ...
}
```

### 8.3 模块级文档

每个 `.rs` 文件的顶部写 `//!` 模块级文档：

```rust
//! Shell command executor.
//!
//! This module handles the execution of a parsed [`Pipeline`]:
//! - Forks child processes for external commands.
//! - Routes built-in commands to the [`builtins`](super::builtins) module.
//! - Manages pipe connections between pipeline stages.
//!
//! # Safety
//!
//! This module uses `fork(2)` and `execve(2)`. See [`fork`] for the
//! restrictions on signal-unsafe operations between fork and exec.
```

### 8.4 文档中的链接

```rust
/// Returns the exit status of this [`Job`].
///
/// If the job is still running, returns [`None`].
///
/// [`Job`]: crate::jobs::Job
pub(crate) fn status(&self) -> Option<i32> { ... }
```

使用 `` [`TypeName`] `` 短链接格式，然后在末尾集中列出引用目标。

### 8.5 文档测试（Doc tests）

````rust
/// ``` 代码块会自动被 `cargo test` 编译和运行。利用这一点确保文档中的示例永远正确。
///
/// ```
/// use osh::Shell;
///
/// let mut shell = Shell::new();
/// let exit_code = shell.run_command("echo hello")?;
/// assert_eq!(exit_code, 0);
/// # Ok::<(), osh::error::ShellError>(())
/// ```
````

---

## 9. 测试

### 9.1 测试金字塔

```text
        /\
       /E2E\         少量端到端测试（完整的 shell 交互）
      /------\
     / 集成  \        中量集成测试（多模块协作）
    /----------\
   /  单元测试   \     大量单元测试（单函数/单方法）
  /--------------\
```

### 9.2 单元测试

- 放在被测试代码的同一个文件中，包裹在 `#[cfg(test)] mod tests { ... }` 里。
- 测试函数命名：`test_<function_name>__<scenario>`（双下划线分隔函数名和场景）。
- 每个测试只验证一件事。宁可多写几个小测试，也不要写一个"把所有东西都测了"的大测试。

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse__empty_input_returns_empty_pipeline() {
        let result = parse("").unwrap();
        assert!(result.commands.is_empty());
    }

    #[test]
    fn test_parse__single_command_no_args() {
        let pipeline = parse("ls").unwrap();
        assert_eq!(pipeline.commands.len(), 1);
        assert_eq!(pipeline.commands[0].name, "ls");
        assert!(pipeline.commands[0].args.is_empty());
    }

    #[test]
    fn test_parse__unmatched_quote_returns_error() {
        let result = parse("echo \"hello");
        assert!(matches!(result, Err(ShellError::Parse(_))));
    }
}
```

### 9.3 集成测试

放在 `tests/` 目录下。每个测试文件是一个独立的二进制，测试完整的端到端流程：

```rust
// tests/shell_integration.rs
use std::process::Command;

#[test]
fn test_shell_executes_simple_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "-c", "echo hello"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
}
```

### 9.4 表驱动测试

对于纯数据转换（parser、serializer），使用表驱动测试减少样板代码：

```rust
#[test]
fn test_word_split() {
    let cases = vec![
        ("", vec![]),
        ("ls", vec!["ls"]),
        ("ls -la", vec!["ls", "-la"]),
        ("echo \"a b\"", vec!["echo", "a b"]),
    ];
    for (input, expected) in cases {
        assert_eq!(split_words(input), expected,
            "failed on input: {input:?}");
    }
}
```

### 9.5 CI 中的测试要求

```makefile
test:
    cargo test --all-targets
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check
```

---

## 10. `unsafe` 代码

### 10.1 最小化原则

- **默认不允许 `unsafe`**。在使用它之前，穷尽所有 safe 替代方案。
- 本项目是 OS lab，与 `libc` 交互是合理的 `unsafe` 使用场景。其他场景（自引用的数据结构、手动内存管理、FFI）需要明确注释。

### 10.2 每次使用 `unsafe` 的检查清单

```rust
// SAFETY: 此处调用 libc::tcgetpgrp 获取当前终端的前台进程组。
// 前提条件：
// 1. fd 是一个已打开的终端文件描述符，由父进程在 fork 前打开。
// 2. 此函数的调用发生在子进程执行 execve 之前，此时仍为单线程。
// 3. 返回值立即检查，不会传播无效的 pid_t。
let pgid = unsafe { libc::tcgetpgrp(fd) };
```

检查项：

1. 是否写明了 **为什么必须用 `unsafe`**？
2. 是否列出了 **所有前置条件**？
3. 前置条件是否 **可以由代码审查者在本地验证**？（即不依赖"运行时不会发生"的推测）
4. `unsafe` 块是否 **尽可能小**？（不要把 safe 的代码也包进去）

### 10.3 封装 `unsafe`

所有 `unsafe` 代码应该藏在安全抽象后面：

```rust
/// 安全封装：获取终端前台进程组 ID。
///
/// # Errors
///
/// 如果 fd 不是有效的终端文件描述符，返回 `ShellError::Syscall`。
pub(crate) fn tcgetpgrp(fd: i32) -> Result<libc::pid_t, ShellError> {
    // SAFETY: fd 由调用者提供，调用者保证它是已打开的终端 fd。
    // 不涉及多线程竞争——shell 在主线程中调用此函数。
    let pgid = unsafe { libc::tcgetpgrp(fd) };
    if pgid == -1 {
        return Err(ShellError::Syscall {
            call: "tcgetpgrp",
            source: io::Error::last_os_error(),
        });
    }
    Ok(pgid)
}
```

---

## 11. 依赖管理

### 11.1 添加依赖前过三关

1. **标准库是否已经提供了？**——`std` 里没有随机数，但它有 `HashMap`。
2. **依赖本身是否健康？**——检查最近更新时间、下载量、issue 数量。
3. **引入的 transitive dependencies 有多少？**——`cargo tree` 看看。

### 11.2 版本固定

- 可执行程序（binary crate）：`Cargo.lock` 必须提交到 git。
- 库（library crate）：在 `Cargo.toml` 中指定兼容版本范围，如 `"0.31"`（等同于 `>=0.31.0, <0.32.0`）。

### 11.3 Feature flag 最小化

- 默认关闭所有 optional feature。`default = []`。
- 每个 feature 在 `Cargo.toml` 上方加注释说明它引入的额外依赖和增加的编译时间。

```toml
# 启用交互式模式（readline 支持，增加约 0.3s 编译时间）
interactive = ["rustyline"]
```

---

## 12. 工程化工作流

### 12.1 提交前检查清单

每次开发完成一个小功能后执行：

```bash
cargo fmt             # 格式化代码
cargo check           # 快速检查编译
```

功能稳定后再运行：

```bash
cargo clippy -- -D warnings  # 零 warning
cargo test --all-targets     # 所有测试通过
cargo doc --no-deps           # 文档构建无警告
```

可以将以上命令放在 `Makefile` 中：

```makefile
.PHONY: check
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo doc --no-deps --document-private-items
    @echo "All checks passed."
```

### 12.2 Commit message 格式

```text
[模块] 简短描述（<50 字符）

可选详细说明（每行 <72 字符）。

修复：链接到 issue 或描述 bug。
```

示例：

```text
[parser] 支持双引号内的转义字符

修复了 "echo \"hello\"" 被错误解析为两个字符串的问题。
双引号内的 \" 现在被正确识别为转义引号。
```

### 12.3 分支策略

- `main`：始终可编译、可通过测试。
- 功能开发：`feature/<name>`（如 `feature/pipe-execution`）。
- Bug 修复：`fix/<issue-id>`。

### 12.4 Clippy 配置

在 `lib.rs` 或 `main.rs` 顶部添加 crate 级 lint 配置：

```rust
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]        // 标记所有 unwrap() 供审查
#![warn(clippy::expect_used)]        // 标记所有 expect() 供审查
#![warn(clippy::todo)]               // 禁止提交 TODO 到 main 分支
```

---

## 13. 性能心态

### 13.1 正确性 > 简洁 > 性能

在你证明某段代码是性能瓶颈之前（通过 `perf`、`flamegraph` 或 `criterion` benchmark），不要牺牲可读性去"优化"它。

### 13.2 对 Shell 项目的具体建议

- 字符串处理用 `&str` 而非 `String` 作为函数参数（见 7.2）。
- `Vec::with_capacity` 预分配已知大小的缓冲区。
- 解析阶段尽量使用 zero-copy（`&str` 切片引用原始输入）。

---

## 14. 快速决策速查表

| 场景 | 选择 |
| --- | --- |
| 两种可能的状态 | `enum`（不用 `bool`） |
| 函数可能失败 | `Result<T, E>`（不 panic） |
| 可恢复 vs 不可恢复错误 | `Result`（除非内存耗尽/数据结构损坏） |
| 需要 clone | 先考虑借用，再考虑 move，最后 clone |
| 有多个不同类型错误 | `thiserror::Error` derive enum |
| 应用层附加错误上下文 | `anyhow::Context` |
| 类型间语义不同 | newtype wrapper |
| 公开 API | 必须写 `///` doc comment + `# Examples` |
| 所有 unsafe | `// SAFETY:` 注释 + 最小化 unsafe 块 |
| 往 main 分支提交 | 必须通过 fmt + clippy + test |
| 字符串参数 | `&str`（不是 `String`） |

---

## 参考资料

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Book - Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [rustdoc Book](https://doc.rust-lang.org/rustdoc/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)
