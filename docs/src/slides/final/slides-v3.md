---
theme: touying
title: DiPECS — 面向 Android/Linux 的本地优先 AIOS 原型系统
info: |
  DiPECS final presentation v3 · 40 minutes
layout: cover
class: text-center
transition: slide-left
duration: 40min
drawings:
  persist: false
mdc: true
touying:
  preset: simple
  footer: DiPECS · Local-First Android/Linux AIOS
---

# DiPECS

## 面向 Android/Linux 的本地优先 AIOS 原型系统

<div class="mt-7 text-lg opacity-80">
感知 · 脱敏 · 决策 · 授权执行 · 回放审计
</div>

<div class="pt-10 text-sm opacity-60">
操作系统课程项目最终汇报 · 40 分钟 · 2026.07
</div>

---

# 一句话理解 DiPECS

<div class="mt-7 grid grid-cols-6 gap-2 text-center text-xs">
  <div class="p-3 border rounded"><div class="text-xl mb-2">①</div><b>Sense</b><br/>应用、通知、进程、设备</div>
  <div class="p-3 border rounded"><div class="text-xl mb-2">②</div><b>Sanitize</b><br/>Privacy Air-Gap</div>
  <div class="p-3 border rounded"><div class="text-xl mb-2">③</div><b>Context</b><br/>结构化时间窗口</div>
  <div class="p-3 border rounded"><div class="text-xl mb-2">④</div><b>Decide</b><br/>本地优先、可选 LLM</div>
  <div class="p-3 border rounded"><div class="text-xl mb-2">⑤</div><b>Authorize</b><br/>策略审查后执行</div>
  <div class="p-3 border rounded"><div class="text-xl mb-2">⑥</div><b>Audit</b><br/>终态记录与回放</div>
</div>

<div class="mt-7 p-5 border rounded text-center text-lg">
探索智能操作系统中<strong>感知—决策—授权执行—审计</strong>的本地可信闭环
</div>

---

# 汇报路线 · 40 min

<div class="mt-6 compact-table text-sm">

| 部分 | 时间 | 回答的问题 |
|---|---:|---|
| 1. 问题与定位 | 5 min | 为什么 AIOS 需要本地可信闭环？它与 OS 有何关系？ |
| 2. 系统机制 | 10 min | `/proc`、daemon、Android 服务、文件流如何工作？ |
| 3. 决策与治理 | 12 min | 如何脱敏、路由、授权并防止模型越权？ |
| 4. 闭环运行证据 | 4 min | 不依赖现场 Demo，如何证明管线和动作可核验？ |
| 5. 实验与边界 | 7 min | 当前数据证明什么、不能证明什么？ |
| 6. 总结 | 2 min | 项目贡献、限制与后续工作 |

</div>

---
layout: section
---

# 01 问题、目标与定位

---

# 从“AI 能力”到“可信 AIOS 闭环”

<div class="grid grid-cols-2 gap-6 mt-6 text-sm">

<div class="p-5 border rounded">

### 系统侧问题

感知信号 → 模型建议 → 系统动作

- 本地信号分散在不同 OS 接口
- 原始通知与行为数据具有隐私风险
- 模型输出不具备系统执行权限
- 网络与模型失败不能阻塞控制路径

</div>

<div class="p-5 border rounded">

### DiPECS 的研究问题

能否把 AI 建议约束在可控的 OS 机制内？

- 本地优先完成脱敏、决策与降级
- 模型只产生 Intent，不直接操作系统
- 动作必须经过授权、执行和终态审计
- 预测错误时允许拒绝和 `NoOp`

</div>

</div>

<div class="mt-6 p-4 border rounded text-center">
核心不是“让模型控制 OS”，而是让模型在<strong>权限、策略与审计边界</strong>内参与系统决策。
</div>

---

# 典型场景：附件通知触发受控资源准备

<div class="mt-7 grid grid-cols-4 gap-3 text-center text-sm">
  <div class="p-4 border rounded"><b>t₀</b><br/><br/>收到聊天应用通知<br/>包含文件语义</div>
  <div class="p-4 border rounded"><b>t₀ + Δ</b><br/><br/>脱敏上下文判断<br/>可能打开该应用</div>
  <div class="p-4 border rounded"><b>策略门</b><br/><br/>检查目标、风险、<br/>能力与置信度</div>
  <div class="p-4 border rounded"><b>t₁</b><br/><br/>授权低风险准备<br/>或安全地 NoOp</div>
</div>

<div class="mt-7 grid grid-cols-3 gap-4 text-sm">
  <div class="p-3 border rounded"><b>隐私约束</b><br/>通知正文不进入模型</div>
  <div class="p-3 border rounded"><b>安全约束</b><br/>模型不能直接调用执行器</div>
  <div class="p-3 border rounded"><b>资源约束</b><br/>低置信度时不执行</div>
</div>

---

# 项目目标与非目标

<div class="grid grid-cols-2 gap-6 mt-5 text-sm">

<div>

### 本项目要验证

- Android 与 Linux 信号能否形成统一上下文
- 原始隐私能否在模型前被强制隔离
- 多级决策能否退化到确定性本地路径
- 动作能否经过策略、认证和审计后执行
- 决策器在可观察上下文中能否给出可评测预测

</div>

<div>

### 当前版本不声称

- 不修改 Linux 内核、调度器或 Android LMKD
- 不把合成预测准确率外推为真实用户效果
- 不保证普通 APK 可执行所有系统级动作
- 不把模拟器功耗估算当作真机实测
- 不把未接入主链路的本地模型实验当成成品

</div>

</div>

---

# 三项核心贡献

<div class="mt-6 grid grid-cols-3 gap-5 text-sm">

<div class="p-5 border rounded">

### 1 · 本地感知上下文

将通知、应用、进程与设备状态标准化；原始文本和路径在 `PrivacyAirGap` 前后严格分界。

</div>

<div class="p-5 border rounded">

### 2 · 本地优先决策

用规则、本地评分和可选云模型覆盖不同复杂度；敏感、失败或离线时安全降级。

</div>

<div class="p-5 border rounded">

### 3 · 授权执行与审计

模型只提出候选；策略引擎、能力上限、HMAC 信封和生命周期共同形成可回放执行记录。

</div>

</div>

<div class="mt-7 p-4 border rounded text-center">
设计原则：<strong>机制与策略分离 · 最小权限 · fail closed · 可回放</strong>
</div>

---
layout: section
---

# 02 系统架构与 OS 机制

---

# 总体架构：本地优先 AIOS 数据面与控制面

<div class="diagram-frame">
  <img src="/diagrams/arch-overview.svg" />
</div>

<div class="mt-2 text-center opacity-75 text-sm">
Collector / Executor 提供机制，Router / Policy 决定策略；云模型不是可信计算基的一部分。
</div>

---

# 部署与进程边界

<div class="mt-5 grid grid-cols-3 gap-5 text-sm">

<div class="p-4 border rounded">

### Android App 进程

- 公开 Android API 采集
- 私有 JSONL 持久化
- localhost action server
- 设备侧动作执行与审计

</div>

<div class="p-4 border rounded">

### `dipecsd` 进程

- `/proc` 与系统状态采集
- 脱敏、窗口、决策、策略
- 动作授权和回执处理
- SIGINT/SIGTERM 优雅退出

</div>

<div class="p-4 border rounded">

### 可选模型服务

- 本地规则默认可独立运行
- LLM 仅接收 Sanitized Context
- 不拥有设备执行权限
- 超时、异常时回退本地
- 默认可完全关闭

</div>

</div>

---

# 模块依赖：稳定协议与入口层解耦

<div class="diagram-frame">
  <img src="/diagrams/1module-deps.svg" />
</div>

<div class="diagram-caption">
`aios-spec` 固化跨模块类型、trait 与 IPC 协议；daemon / CLI 只是组合入口，不反向污染核心库。
</div>

---

# OS 相关性：接口、问题、原则

<div class="mt-4 compact-table text-sm">

| 层次 | DiPECS 中的实现 | OS 课程关联 |
|---|---|---|
| 进程 | `/proc` 快照差分、`fork + setsid`、signal | 进程状态、daemon、生命周期 |
| IPC | localhost socket、请求/响应、超时 | 进程间通信、阻塞与失败语义 |
| 文件 | append-only JSONL、positional tail、flush | 文件偏移、追加语义、持久化 |
| 资源 | PreWarm / KeepAlive / Prefetch / Release | 进程、内存、I/O 资源管理 |
| 安全 | capability、policy、HMAC、审计 | 权限分离、最小权限、引用监控器 |
| Android | UsageStats、通知服务、JobScheduler | OS 服务与受控系统接口 |

</div>

<div class="mt-5 p-4 border rounded text-center text-sm">
定位：<strong>本地优先的用户态 AIOS 原型</strong>，以 OS 机制约束智能决策，而不是修改内核算法。
</div>

---

# `/proc`：进程状态的内核接口

<div class="grid grid-cols-2 gap-6 mt-5 text-sm">

<div>

`ProcReader::scan_all()` 周期读取：

```text
/proc/<pid>/status
  VmRSS       常驻内存
  VmSwap      换出内存
  Threads     线程数
  Uid         所属用户

/proc/<pid>/oom_score
```

</div>

<div class="p-4 border rounded">

### 为什么做快照差分？

1. 本轮扫描构造 `pid → ProcSnapshot`
2. 与上一轮快照比较
3. 只为变化的进程生成事件
4. 减少后续窗口中的冗余信号

阈值规则还能识别高 RSS 或高 Swap 的内存压力候选。

</div>

</div>

---

# Daemon：进程与会话管理

<div class="diagram-frame">
  <img src="/diagrams/1runtime-pipeline.svg" />
</div>

<div class="diagram-caption">
`fork + setsid + /dev/null` 完成 daemon 化；容量 4096 的 channel 解耦采集与处理，退出前刷新剩余窗口。
</div>

---

# Android OS 服务作为受控事件源

<div class="mt-4 compact-table text-sm">

| 数据源 | 事件 | 权限 / 约束 |
|---|---|---|
| `UsageStatsManager` | 应用前后台切换 | Usage Access app-op |
| `NotificationListenerService` | 通知发布与交互 | 用户显式启用监听 |
| `AccessibilityService` | 过滤后的 UI 信号 | 用户显式授权；不作为主链原文源 |
| Foreground Service | 5 s 轮询与 30 s heartbeat | 持续通知、生命周期约束 |
| `JobScheduler` | KeepAlive 维护任务 | 异步执行、系统统一调度 |

</div>

<div class="mt-5 p-4 border rounded text-sm">
选择公开 API 的原因：无需内核 hook，权限边界清晰，模拟器和真机都可复现。
</div>

---

# Append-only JSONL 与增量 tail

<div class="diagram-frame">
  <img src="/diagrams/rust-ingress.svg" />
</div>

<div class="diagram-caption">
同一份 trace 同时服务于在线输入、离线 replay、回归测试与审计取证。
</div>

---

# Replay / Audit：在线和离线共用核心逻辑

<div class="diagram-frame">
  <img src="/diagrams/replay-audit.svg" />
</div>

<div class="diagram-caption">
在线与离线只在适配器处不同；核心状态迁移和审计格式保持一致。
</div>

---
layout: section
---

# 03 隐私、决策与动作治理

---

# Privacy Air Gap：模型前的强制边界

<div class="diagram-frame">
  <img src="/diagrams/privacy-boundary.svg" />
</div>

<div class="diagram-caption">
原始区与安全区以类型和依赖方向隔离；模型、上下文和审计只消费 `SanitizedEvent`。
</div>

---

# 通知文本脱敏：从原始通知到语义提示

<div class="diagram-frame">
  <img src="/diagrams/semantic-hints.svg" />
</div>

<div class="diagram-caption">
输出只保留统计量和类别提示；原文、路径、键值不进入模型输入。
</div>

---

# 10 秒上下文窗口

<div class="diagram-frame">
  <img src="/diagrams/window-aggregation.svg" />
</div>

<div class="diagram-caption">
窗口结束后才触发一次决策；避免为每个噪声事件调用模型。
</div>

---

# DecisionRouter：分级路由与熔断

<div class="diagram-frame">
  <img src="/diagrams/decision-routing.svg" />
</div>

<div class="diagram-caption">
路由优先级：熔断状态 → 隐私敏感度 → 本地可动作信号 → 语义复杂度。连续错误按时间窗口计数，成功后清零。
</div>

---

# LocalEvaluator：可解释的确定性评分

<div class="mt-4 grid grid-cols-2 gap-6 text-sm">

<div>

```text
score = base
      + foreground_match
      + notification_file_correlation
      + repeated_package
      + behavior_memory
      - low_battery
      - cellular_or_offline
      - recent_policy_rejection
      - recent_execution_failure
```

</div>

<div class="p-4 border rounded">

### 输出约束

- 每窗口最多 5 个 intent
- 所有分数裁剪到 `[0, 1]`
- 只产生能力表允许的低风险动作
- 历史反馈只调整有限幅度
- 结果可重复，不依赖随机采样

</div>

</div>

---

# PolicyEngine：模型建议不是权限

<div class="diagram-frame">
  <img src="/diagrams/policy-check-flow.svg" />
</div>

<div class="diagram-caption">
八项检查按顺序 fail closed；只有全部通过才产生 `PolicyActionDecision::Approved`。
</div>

---

# ActionLifecycle：每个动作恰好一个终态

<div class="diagram-frame">
  <img src="/diagrams/action-lifecycle.svg" />
</div>

<div class="diagram-caption">
只有 lifecycle 能构造 `AuthorizedAction`；连接、超时、设备拒绝与非法回执均进入可审计终态。
</div>

---

# Android Action Bridge：认证与防重放

<div class="mt-4 grid grid-cols-2 gap-5 text-sm">

<div>

```text
BridgeExecuteRequest
├── message_type = execute
├── issued_at_ms
├── expires_at_ms = issued + 60 s
├── action = canonical AuthorizedAction JSON
└── auth.hmac_sha256
```

</div>

<div class="p-4 border rounded">

### HMAC 覆盖范围

```text
protocol version
+ issued_at_ms
+ expires_at_ms
+ length(action)
+ action bytes
```

因此旧标签不能替换动作，也不能跨过 freshness window 重放。

</div>

</div>

<div class="mt-5 text-center text-sm opacity-75">
服务只绑定 `127.0.0.1:46321`，并对负载大小、读超时、并发队列和连续认证失败做限制。
</div>

---

# 动作与权限边界

<div class="mt-4 compact-table text-sm">

| Action | 设备侧实现 | 普通 APK / system app 边界 |
|---|---|---|
| `PreWarmProcess` | 启动目标 Activity，轮询任务后清理可见 task | 后台启动第三方应用受限；系统签名更完整 |
| `PrefetchFile` | HTTPS / URI 预取到受控缓存 | 普通 APK 可执行自身可访问资源 |
| `KeepAlive` | OOM/cgroup 尝试 + JobScheduler fallback | OOM/cgroup 写入通常需要特权 |
| `ReleaseMemory` | 清理预取缓存；尝试包缓存与 page cache | 跨应用/全局回收需要系统权限 |
| `NoOp` | 确定性本地终态 | 安全退化路径 |

</div>

<div class="mt-5 p-4 border rounded text-sm">
同一动作在普通 APK 环境可能退化、被拒绝或只作用于自身资源；这属于 Android 权限模型，而不是隐藏为“成功”。
</div>

---
layout: section
---

# 04 闭环运行证据

---

# 案例输入：本地事件如何进入系统

<div class="diagram-frame">
  <img src="/diagrams/data-flow-e2e.svg" />
</div>

<div class="diagram-caption">
模拟器实采 E2E：Android 35，数据源为 `EMULATOR-MEASURED / NON-SYNTHETIC`；事件进入 replay，并生成审计哈希 `sha256:c99c471c…16d7`。
</div>

---

# 案例裁决：允许与拒绝

<div class="diagram-frame">
  <img src="/diagrams/action-governance.svg" />
</div>

<div class="diagram-caption">
高置信度不能绕过上下文事实；Policy 是独立于模型的硬边界。
</div>

---

# 设备执行证据：四类处理器均有终态

<div class="mt-4 compact-table text-sm">

| 动作 | 设备终态审计事件 | 确认延迟 | 结果 |
|---|---|---:|---|
| KeepAlive | `keep_alive_scheduled → job_executed` | 21.3 ms | EXECUTED |
| ReleaseMemory | `release_memory_completed` | 13.4 ms | EXECUTED |
| PreWarmProcess | `own_resources_prewarmed` | 31.2 ms | EXECUTED* |
| PrefetchFile | `prefetch_started → prefetch_succeeded` | 1.1 ms* | EXECUTED |

</div>

<div class="mt-5 grid grid-cols-2 gap-4 text-xs">
  <div class="p-3 border rounded">设备记录 `authorized_action_socket_execute_ok = 4`，四类处理器均留下终态。</div>
  <div class="p-3 border rounded">* PreWarm 验证 `own:warmup`；Prefetch 回执仅表示入队，最终下载由 `prefetch_succeeded` 取证。</div>
</div>

<div class="mt-5 text-xs opacity-70">
诚实边界：设备证据经与生产信封逐字节一致的取证发送器通过 adb forward 获得；它证明设备处理器执行，不等价于 daemon 已在设备内生产部署。Rust `AndroidAdapter` 另有 mock-socket E2E。
</div>

---
layout: section
---

# 05 实验设计、结果与边界

---

# 实验问题与证据层级

<div class="mt-4 compact-table text-sm">

| RQ | 问题 | 证据 |
|---|---|---|
| RQ1 | 管线能否从 Android 事件走到可复现审计？ | 模拟器实采 E2E + replay hash |
| RQ2 | 隐私边界是否阻止原始通知泄漏？ | naive prompt 与 DiPECS 对照 |
| RQ3 | 本地决策为何比云端更适合即时路径？ | 规则、本地、真实 API 延迟 |
| RQ4 | 当前决策器能否预测上下文支持的下一应用？ | 合成 trace + 派生 ground truth |
| RQ5 | 常驻开销是否可控？ | emulator CPU/RSS/PSS + replay 吞吐 |
| RQ6 | 动作治理是否覆盖主要拒绝路径？ | Policy 20 项测试 + action audit |

</div>

<div class="mt-5 text-xs opacity-75">
证据标签：真实 API、模拟器实测、离线 replay、估算值分别标注，不混为同一强度结论。
</div>

---

# 合成预测评测：能力与覆盖边界

<div class="mt-4 grid grid-cols-2 gap-6 text-sm">

<div>

### 数据与标签

- 3 个确定性合成场景
- 10 s 上下文窗口，30 s 预测 horizon
- 总窗口：946
- 有未来切换：764
- 上下文可支持标签：178（23.3%）

</div>

<div class="compact-table">

| 后端 | Top-1 | Top-3 | 预测覆盖 |
|---|---:|---:|---:|
| RuleBased | **61.2%** | 65.7% | **93.8%** |
| LocalEvaluator | 43.8% | **62.9%** | 73.6% |

</div>

</div>

<div class="mt-5 p-4 border rounded text-sm">
结论边界：结果只描述<strong>合成、上下文可观察</strong>的切换；它验证评测框架与策略行为，不代表真实用户泛化准确率。RuleBased 的条件错误预测率仍为 34.7%。
</div>

---

# 决策延迟：本地即时，云端非即时

<div class="mt-5 compact-table text-sm">

| 后端 | p50 | p95 | 路径定位 |
|---|---:|---:|---|
| RuleBased | 0.00 ms | 0.02 ms | 高频、低风险 |
| LocalEvaluator | 0.01 ms | 0.05 ms | 无网络、可解释评分 |
| Cloud LLM（真实 API，10 轮） | 7339.6 ms | 10050.1 ms | 复杂语义、非即时 |

</div>

<div class="mt-6 p-4 border rounded text-sm">
工程结论：即时资源动作默认本地；云端只适合作为可选分析路径，不能成为关键控制回路依赖。当前数据只证明延迟，不证明云端决策质量。
</div>

---

# 隐私与治理结果

<div class="mt-4 grid grid-cols-2 gap-6 text-sm">

<div class="p-4 border rounded">

### Privacy comparison

| 指标 | Naive | DiPECS |
|---|---:|---:|
| 原始文本泄漏 | 22 | **0** |
| 输入大小 | 63,178 B | **645 B** |

审计流与 NDJSON 泄漏测试：2/2 pass。

</div>

<div class="p-4 border rounded">

### Governance coverage

- PolicyEngine：20 项测试通过
- target-in-context
- risk / capability 双重上限
- confidence floor
- batch cap 与 deferred filter
- FallbackNoOp 能力隔离

</div>

</div>

---

# 资源开销与离线吞吐

<div class="mt-4 grid grid-cols-2 gap-6 text-sm">

<div>

### Android emulator · 30 samples/mode

| 模式 | Avg PSS | 相对基线 |
|---|---:|---:|
| baseline | 36.024 MB | — |
| observe only | 39.629 MB | +3.605 MB |
| action loop | 41.621 MB | +5.597 MB |

</div>

<div>

### 2400-line replay

- 有效事件：1,631
- 完成窗口：58
- Wall time：128 ms
- Peak RSS：10.77 MB
- 吞吐：12,742 events/s
- 授权动作：206

</div>

</div>

<div class="mt-5 text-xs opacity-70">
CPU 采样存在粒度噪声；电池和温度为模拟器估算，本报告不将其作为实测功耗结论。
</div>

---

# 负面结果与有效性威胁

<div class="mt-5 grid grid-cols-2 gap-5 text-sm">

<div class="p-4 border rounded">

### 当前数据不支持

- “预热快 43.8%”的因果结论：脚本混入 cold / warm process 差异
- ReleaseMemory 有效降低 PSS：结果反而增加 0.331 MB
- 模拟器电池/温度代表真机功耗
- Cloud LLM 能稳定产生有效即时动作
- 合成准确率代表真实用户行为

</div>

<div class="p-4 border rounded">

### 因此我们的表述

- 启动时间数据不进入当前正面结论，待独立 controller 重测
- ReleaseMemory 仅视为链路覆盖，不视为收益证明
- 功耗结果标为 estimated
- 云端视为非即时可选后端
- 预测结果同时报告 eligible coverage 与错误预测率

</div>

</div>

---
layout: section
---

# 06 局限、未来工作与总结

---

# 项目边界：为什么是“用户态控制平面”

<div class="mt-5 grid grid-cols-2 gap-6 text-sm">

<div class="p-4 border rounded">

### 已实现

- 使用 OS 暴露的进程、文件、IPC 和服务接口
- 在用户态实现机制/策略分离
- 建立隐私、能力、策略、认证和审计边界
- 在模拟器跑通设备动作和预热实验

</div>

<div class="p-4 border rounded">

### 未深入内核

- 没有修改 scheduler、LMKD、VFS 或 Binder driver
- 普通 APK 受 Android 沙箱与后台启动限制
- Binder/fanotify 探针仍依赖特权部署
- 系统级动作需要 platform signing / ROM 集成

</div>

</div>

<div class="mt-6 p-4 border rounded text-center">
课程价值在于：用 OS 原则设计一个<strong>可运行、可降级、可追责</strong>的资源管理控制回路。
</div>

---

# 下一步：从原型到系统研究

<div class="mt-5 grid grid-cols-3 gap-4 text-sm">

<div class="p-4 border rounded">

### 评测

- 构建真实用户、匿名化 ground truth 数据集
- Top-1 / Top-3、错误预热率
- 用独立 controller 重做启动实验
- 真机功耗和长期稳定性

</div>

<div class="p-4 border rounded">

### OS 集成

- 接入 LMKD / cgroup 反馈
- 使用 Binder 或 Unix domain socket
- platform-signed system app

</div>

<div class="p-4 border rounded">

### 决策

- 接入端侧轻量模型并与规则基线对照
- 在线反馈校准置信度
- 加入资源预算与预热撤销机制

</div>

</div>

---

# 总结

<div class="mt-7 grid grid-cols-3 gap-5 text-sm">
  <div class="p-5 border rounded"><div class="text-2xl mb-3">01</div><b>本地 AIOS 闭环</b><br/><br/>Android / Linux 信号进入统一管线，并形成可回放审计。</div>
  <div class="p-5 border rounded"><div class="text-2xl mb-3">02</div><b>授权而非直控</b><br/><br/>模型不接触原始隐私，也不能绕过本地策略直接执行。</div>
  <div class="p-5 border rounded"><div class="text-2xl mb-3">03</div><b>可评测且诚实</b><br/><br/>报告合成预测、动作终态和开销，同时公开覆盖限制与负面结果。</div>
</div>

<div class="mt-8 p-5 border rounded text-center text-lg">
DiPECS 为 AIOS 提供一种机制策略分离、可降级、可回放、可审计的用户态实践方案。
</div>

---

# Q & A

<div class="mt-10 text-xl opacity-75 text-center">
谢谢
</div>

<div class="mt-10 grid grid-cols-2 gap-4 text-sm opacity-80">
  <div class="p-3 border rounded">为什么不用纯规则或纯云端？</div>
  <div class="p-3 border rounded">预测错误时如何控制代价？</div>
  <div class="p-3 border rounded">普通 APK 能执行哪些动作？</div>
  <div class="p-3 border rounded">为什么这属于 OS 课程项目？</div>
</div>
