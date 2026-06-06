# llama.cpp 与 GGUF 模型接入

## 结论

正式 Lab4 实验使用 `llama.cpp` 完成真实模型推理、参数优化和 RPC 分布式推理；本仓库的 Rust 代码负责 prompt 校验、命令封装、JSONL 记录、结果汇总和 Ceph 存储测量。

不要把 `llama.cpp` 源码复制进仓库，也不要把 GGUF 大模型提交进 Git。

## 目录约定

```text
lab4/
├── third_party/
│   └── llama.cpp/          # git submodule，只提交指针
├── data/
│   ├── models/             # 本地 GGUF 模型，gitignore
│   ├── prompts/
│   ├── raw/
│   └── results/
└── docs/
```

## 拉取 llama.cpp

推荐使用 submodule：

```bash
git submodule add https://github.com/ggml-org/llama.cpp lab4/third_party/llama.cpp
git submodule update --init --recursive
```

如果已经存在 submodule 记录，其他机器只需要：

```bash
git submodule update --init --recursive
```

固定版本：

```bash
cd lab4/third_party/llama.cpp
git rev-parse HEAD
```

把 commit hash 写入 `lab4/reports/deployment.md`，避免报告只有“最新版”这种不可复现描述。

## 构建 llama.cpp

### 环境准备

需要 `cmake` 和 `clang`（或兼容的 gcc）。**推荐 Clang**：GCC 16+ 对当前 llama.cpp 某些版本存在兼容性警告（如方法调用语法检查更严格），用 Clang 可直接避免。

Ubuntu/Debian 示例：

```bash
sudo apt install cmake clang
```

### CPU + RPC 构建

```bash
cd lab4/third_party/llama.cpp
rm -rf build

CC=clang CXX=clang++ cmake -B build -S . -DCMAKE_BUILD_TYPE=Release -DGGML_RPC=ON
CC=clang CXX=clang++ cmake --build build --config Release -j "$(nproc)"
```

验证可执行文件：

```bash
./build/bin/llama-cli --version
./build/bin/llama-bench --help
./build/bin/rpc-server --help
```

如果机器有 NVIDIA GPU，可另建一个 CUDA build 目录：

```bash
cd lab4/third_party/llama.cpp
cmake -B build-cuda -S . -DCMAKE_BUILD_TYPE=Release -DGGML_CUDA=ON -DGGML_RPC=ON
cmake --build build-cuda --config Release -j "$(nproc)"
```

不要创建 Makefile。课程实验命令和本仓库自动化入口统一使用 Cargo、CMake 和文档命令。

## 下载 GGUF 模型

当前后续实验使用 Qwen3.5-2B 的 Q4_K_M 社区量化模型。此前 Qwen2.5-1.5B 的
结果保留为历史基线，不与 Qwen3.5 数据直接混合。

下载模型：

```bash
mkdir -p lab4/data/models

wget --show-progress -c \
  "https://huggingface.co/bartowski/Qwen_Qwen3.5-2B-GGUF/resolve/main/Qwen_Qwen3.5-2B-Q4_K_M.gguf?download=true" \
  -O lab4/data/models/qwen3.5-2b-q4_k_m.gguf
```

校验当前文件：

```bash
sha256sum lab4/data/models/qwen3.5-2b-q4_k_m.gguf
# 57a1085840f497d764a7fc5d346922dbde961efb54cc792ea81d694fd846a1d8
```

为了实验可复现，更推荐显式下载 `.gguf` 到 `lab4/data/models/`，然后用 `-m` 指定路径。

## 单机 smoke 测试

```bash
lab4/third_party/llama.cpp/build/bin/llama-cli \
  -m lab4/data/models/qwen3.5-2b-q4_k_m.gguf \
  -p "用中文解释什么是操作系统页缓存。" \
  -n 128 \
  --threads 12 \
  --reasoning off \
  --reasoning-budget 0
```

使用 Rust 工具记录结果：

```bash
cargo run --manifest-path lab4/Cargo.toml -p lab4-tools --bin lab4-bench -- \
  --prompts lab4/data/prompts/quality-prompts.jsonl \
  --executable lab4/third_party/llama.cpp/build/bin/llama-cli \
  --model lab4/data/models/qwen3.5-2b-q4_k_m.gguf \
  --output lab4/data/results/single-quality.jsonl \
  --mode single \
  --case-prefix single-quality \
  --arg=--threads --arg=12 \
  --arg=--reasoning --arg=off \
  --arg=--reasoning-budget --arg=0
```

## RPC smoke 测试

双机部署涉及 WSL2 NAT/镜像网络、Windows 防火墙、CPU/CUDA worker 构建和
单机/RPC 对照测试。完整流程见
[RPC 双机操作手册](rpc-two-machine-setup.md)。

`rpc-server` 没有认证和传输加密，不要暴露到公网，只在可信局域网中运行。

## 记录要求

每次正式实验至少记录：

- `llama.cpp` submodule commit hash。
- 构建参数，如 `-DGGML_RPC=ON`、`-DGGML_CUDA=ON`。
- 模型 repo、文件名、量化格式、文件大小。
- 单机命令、RPC 主从机命令。
- JSONL 结果路径。
- 机器硬件、OS、内核和网络环境。

## 参考来源

- `llama.cpp` 官方构建文档：<https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md>
- `llama.cpp` RPC 文档：<https://github.com/ggml-org/llama.cpp/blob/master/tools/rpc/README.md>
- Qwen3.5-2B Q4_K_M GGUF：<https://huggingface.co/bartowski/Qwen_Qwen3.5-2B-GGUF/blob/main/Qwen_Qwen3.5-2B-Q4_K_M.gguf>
