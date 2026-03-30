# DiPECS

``` text
aios-root/
├── .github/               # 【必需：CI/CD】
│   ├── workflows/         # 自动化构建、Rust Lint、Android 交叉编译流水线
│   └── pull_request_template.md # 强制代码审查模板 (必须说明状态机变更)
├── crates/                # 【核心：Cargo Workspace】
│   ├── aios-spec/         # 1. 协议定义 (Protobuf/IDL) - 系统的“宪法”
│   ├── aios-kernel/       # 2. 内核拦截器 (eBPF/LSM) - 系统的“触手”
│   ├── aios-core/         # 3. 动作总线与策略引擎 (Rust) - 系统的“脊梁”
│   ├── aios-agent/        # 4. 智能体运行时 (Inference/RAG) - 系统的“大脑”
│   └── aios-adapter/      # 5. 抽象适配层 (Offline/Android 切换开关)
├── data/                  # 【必需：数据资产】
│   ├── traces/            # 原始用户操作轨迹 (Raw JSON/Bin)
│   ├── evaluation/        # 评测集 (Ground Truth)
│   └── schemas/           # 数据清洗与标注的规范
├── docs/                  # 【必需：架构文档】
│   ├── architecture/      # 模块详细设计 (SDDs)
│   ├── rfc/               # 请求评议文档 (每个重构必须先写 RFC)
│   └── api/               # 自动生成的 API 文档 (mdbook)
├── scripts/               # 【必需：运维脚本】
│   ├── setup-env.sh       # 一键配置 Rust + Android NDK 环境
│   ├── collect-trace.py   # 从手机导出用户轨迹的工具
│   └── bench-models.sh    # 模型推理性能压力测试
├── tools/                 # 【必需：可观测性与调试 - 系统的“显微镜”】
│   ├── aios-viz/          # 实时动作流可视化界面 (Web-based)
│   ├── aios-top/          # 类似 top 的实时资源与意图监控器
│   └── aios-replay/       # 离线回放与单步调试器
└── tests/                 # 【必需：测试验证】
    ├── integration/       # 跨模块集成测试
    └── scenarios/         # 场景化测试 (例如：模拟一次完整的“订票”意图)
```

lsy到此一游
