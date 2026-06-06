# Lab4 部署说明

## 总览

本文记录 `llama.cpp` 单机部署、RPC 分布式部署和 Ceph 扩展环境。每条命令都应保留运行机器、工作目录和输出路径，便于复现实验。`lab4-llama` 只用于 Rust smoke，不作为正式推理结果。

## 分项一：硬件与系统环境

环境采集命令：

```bash
cargo run --manifest-path lab4/Cargo.toml -p lab4-tools --bin lab4-env
```

| 节点 | 角色 | CPU | 内存 | GPU | OS / Kernel | IP | 备注 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| archlinux | 主机 | Intel Core i7-13700H，14C/20T | 15,981,548 KiB | Intel Iris Xe + NVIDIA RTX 4060 Laptop | Arch Linux / 7.0.11-arch1-1 | 按实验网络填写 | 当前实验使用 CPU 后端 |
| host-b | 从机 | 待填写 | 待填写 | 待填写 | 待填写 | 待填写 | 运行 `rpc-server` |

## 分项二：模型信息

| 用途 | 模型 | 量化 | 文件大小 | SHA-256 |
| :--- | :--- | :--- | ---: | :--- |
| 已完成的单机基线 | Qwen2.5-1.5B-Instruct | Q4_K_M | 1,117,320,736 bytes | `6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e` |
| 当前 RPC 与后续重跑 | Qwen3.5-2B | Q4_K_M | 1,396,198,496 bytes | `57a1085840f497d764a7fc5d346922dbde961efb54cc792ea81d694fd846a1d8` |

当前模型路径为 `lab4/data/models/qwen3.5-2b-q4_k_m.gguf`。模型文件由
`.gitignore` 排除，只提交路径约定、校验值和实验结果，不提交权重。两种模型的
性能结果不能直接混入同一参数对比表。

## 分项三：llama.cpp 单机部署

拉取第三方依赖：

```bash
git submodule update --init --recursive
cd lab4/third_party/llama.cpp
git rev-parse HEAD
# c4a278d68efa17811006f2123a84081dac03fac7
cmake -B build -S . -DCMAKE_BUILD_TYPE=Release -DGGML_RPC=ON
cmake --build build --config Release -j "$(nproc)"
```

当前固定版本：

| 项目 | 值 |
| :--- | :--- |
| llama.cpp commit | `c4a278d68efa17811006f2123a84081dac03fac7` |
| llama.cpp build | `b9533-c4a278d68` |
| 构建后端 | CPU，已启用 RPC 组件 |
| 模型磁盘 | 本地 NVMe SSD |

Qwen2.5 历史单机基线命令：

```bash
lab4/third_party/llama.cpp/build/bin/llama-cli \
  -m lab4/data/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  -p "用中文解释什么是操作系统页缓存。" \
  -n 128 \
  --threads 8
```

Qwen2.5 历史批量测量命令：

```bash
cargo run --manifest-path lab4/Cargo.toml -p lab4-tools --bin lab4-bench -- \
  --prompts lab4/data/prompts/quality-prompts.jsonl \
  --executable lab4/third_party/llama.cpp/build/bin/llama-cli \
  --model lab4/data/models/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  --output lab4/data/results/single-quality.jsonl \
  --mode single \
  --case-prefix single-quality \
  --arg=--threads --arg=8
```

## 分项四：RPC 分布式部署

完整部署步骤见
[`lab4/docs/rpc-two-machine-setup.md`](../docs/rpc-two-machine-setup.md)。正式实验后
在本节补充实际 IP、网络类型、worker 后端、启动日志和截图。

从机最小命令：

```bash
lab4/third_party/llama.cpp/build-rpc-cpu/bin/rpc-server \
  --host 0.0.0.0 \
  --port 50052 \
  --threads "$(nproc)" \
  --cache
```

主机 Qwen3.5-2B smoke：

```bash
lab4/third_party/llama.cpp/build/bin/llama-cli \
  -m lab4/data/models/qwen3.5-2b-q4_k_m.gguf \
  -p "RPC 分布式推理会引入哪些网络开销？" \
  -n 64 \
  --threads 12 \
  --ctx-size 1024 \
  --batch-size 64 \
  --reasoning off \
  --reasoning-budget 0 \
  --n-gpu-layers all \
  --rpc <WORKER_IP>:50052
```

## 分项五：Ceph 环境

| 字段 | 内容 |
| :--- | :--- |
| 部署方式 | 待填写 |
| Monitor 数量 | 待填写 |
| OSD 数量 | 待填写 |
| Pool / CephFS | 待填写 |
| 副本数 | 待填写 |
| 挂载或对象路径 | 待填写 |

存储测量命令示例：

```bash
cargo run --manifest-path lab4/Cargo.toml -p lab4-tools --bin lab4-storage -- read \
  --case-id local-model-read-001 \
  /path/to/model.gguf \
  --output lab4/data/results/storage-local-read.jsonl

cargo run --manifest-path lab4/Cargo.toml -p lab4-tools --bin lab4-storage -- read \
  --case-id ceph-model-read-001 \
  /mnt/ceph/model.gguf \
  --output lab4/data/results/storage-ceph-read.jsonl
```
