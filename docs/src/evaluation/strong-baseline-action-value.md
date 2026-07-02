# 强 Baseline 与动作收益评估准则

> Status: Planned  
> Last updated: 2026-07-02  
> Purpose: 防止后续评估退回到 toy baseline，明确 DiPECS 要证明的系统价值。

## 结论先行

DiPECS 的主贡献不应表述为“比简单规则更会预测下一应用”，而应表述为：

> 在强预测基线已经存在的前提下，DiPECS 将上下文预测转化为受控、低开销、可执行的 OS 动作闭环，并在启动延迟、动作延迟、内存压力、jank 和稳定性上产生可测收益。

因此，主实验不能只比较 native idle / no-action，也不能把“只预测但不执行”作为核心消融。真正有价值的链路是：

```text
高质量预测 -> 及时动作 -> 设备侧终态 -> 系统指标改善
```

少一环都不能证明本项目的系统价值。

## 主 Baseline

主对照组应是 `StrongPredictiveActionBaseline`，代表“当前已有技术在本应用场景下的强水平”，而不是原生无动作系统。

`StrongPredictiveActionBaseline` 必须满足：

- 使用 Android 可获得的强信号：UsageStats、最近前台应用、最近通知、时间段、应用使用频率、shortcut / intent history 等。
- 使用强预测器：至少包含 recency/frequency、per-current-app majority、Markov；若要正面对标论文，应加入 Transformer / LSTM / MAPLE / Appformer 风格的离线预测器。
- 输出 Top-k 预测，至少报告 Top-1 / Top-3 / Top-5。
- 在与 DiPECS 相同动作预算、相同设备、相同 trace、相同测量脚本下执行 PreWarm / Prefetch / ReleaseMemory。
- 记录执行后的真实系统收益，而不是只记录预测准确率。

这个 baseline 回答的问题是：

> 如果已有强预测技术也能驱动动作，DiPECS 的系统控制面是否仍然带来更好的收益、稳定性或错误动作控制？

## 实验矩阵

主结果表至少包含三列：

| 系统 | 预测来源 | 动作执行 | 用途 |
| --- | --- | --- | --- |
| `StrongPredictiveActionBaseline` | 强启发式或强模型 Top-k | 直接按动作预算执行 | 当前已有技术强水平 |
| `DiPECS Full Loop` | DiPECS context / router / backend | PolicyEngine + ActionLifecycle + Android bridge | 本项目系统创新 |
| `Oracle Upper Bound` | ground truth future app/action | 最优动作时机 | 上限参考，不参与胜负叙事 |

`Native no-action` 可以保留，但只能作为 sanity lower bound，不能作为主 baseline。

## 主指标

预测指标仍然重要，但必须和动作收益一起报告：

| 指标 | 说明 |
| --- | --- |
| `top1_acc` / `top3_acc` / `top5_acc` | 预测质量；Top-5 高不等于系统收益高。 |
| `prewarm_hit_rate` | 被预热应用实际被打开的比例。 |
| `wasted_prewarm_rate` | 预热后未命中的比例，代表资源浪费。 |
| `startup_total_time_delta_ms` | 启动时间改善，必须隔离 cold/warm 干扰。 |
| `action_latency_ms` | intent 生成到设备侧确认动作的延迟。 |
| `memory_pressure_delta` | ReleaseMemory 前后 PSS/RSS/available memory 的变化。 |
| `jank_delta` | 动作前后 frame jank 变化。 |
| `control_plane_overhead` | DiPECS 常驻 CPU / RSS / PSS 开销。 |
| `action_success_rate` | 授权动作到设备终态的成功率。 |
| `net_benefit` | saved latency / memory benefit 减去 wasted action cost 和控制面开销。 |

主结论必须以联合指标成立：

```text
预测命中率足够高，并且动作及时执行，并且系统指标改善超过动作和控制面成本。
```

## 真价值与伪需求

### 真价值

- 强预测下，动作收益仍然可测。
- 同等 Top-k 水平下，DiPECS 的动作闭环更稳定、延迟更低或错误动作代价更小。
- 同等动作预算下，DiPECS 带来更低启动时间、更少 jank、更可控内存压力或更高终态成功率。
- 错误预测时，context window、PolicyEngine、ActionLifecycle 能减少越权动作和无效动作造成的代价。
- Android 设备上能形成真实闭环：intent -> AuthorizedAction -> bridge -> handler -> terminal audit。

### 伪需求

- 只证明能发动作，但不证明性能收益。
- 只证明 Top-k 准，但不执行动作。
- 只和 native idle / no-action 比，并把结果当成主创新。
- PreWarm 实验混入 cold/warm process 差异，却声称因果收益。
- ReleaseMemory 没有稳定降低内存或 jank，却声称优化有效。
- 把隐私和审计作为主优化贡献。它们是系统控制面的可信部署支撑，不是主价值指标。

## 消融准则

消融必须围绕真价值，而不是为了列满表格。

| 消融 | 需要回答的问题 |
| --- | --- |
| `Full DiPECS` | 完整系统收益与成本。 |
| `No Context Window` | 单事件决策是否提高误动作或降低命中率。 |
| `No Dynamic Routing` | 固定路由是否在收益、延迟、错误动作上劣化。 |
| `No Policy Gate` | 无治理动作是否更容易越权、浪费或进入错误执行路径。 |
| `No PreWarm` | 启动收益是否消失。 |
| `No ReleaseMemory` | 内存压力或 jank 指标是否变差；若不变，则 ReleaseMemory 不应作为正面贡献。 |
| `No Android Bridge` | 只有离线 replay 是否无法证明设备端收益。 |

“纯预测不执行动作”不作为主消融。它最多作为预测上限或诊断图表，用于解释动作收益不足是否来自预测质量。

## 结论写法

推荐写法：

> DiPECS does not claim that a lightweight rule backend outperforms all next-app predictors. Instead, it evaluates whether strong contextual prediction can be converted into governed OS actions with measurable system-level benefit under Android constraints.

中文报告中建议写成：

> DiPECS 不主张在纯 next-app Top-k 上击败所有强模型；本项目评估的是，在强预测 baseline 已存在的条件下，系统控制面能否把预测稳定转化为设备动作收益，并控制错误动作、延迟和常驻开销。

