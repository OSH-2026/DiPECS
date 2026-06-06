# llama.cpp RPC 双机操作手册

## 目标与角色

本手册用于完成课程要求的双机 RPC 推理。两台机器的角色固定如下：

| 名称 | 运行环境 | 职责 |
| :--- | :--- | :--- |
| `main-host` | 当前 Linux 主机 | 保存 GGUF、运行 `llama-cli`/`llama-bench`/Rust 工具、保存结果 |
| `rpc-worker` | WSL2 Ubuntu 或原生 Ubuntu | 运行 `rpc-server`，向主机提供 CPU 或 NVIDIA GPU |

数据流：

```text
Qwen3.5 GGUF
    |
    v
main-host: llama-cli / llama-bench / lab4-bench
    |
    | TCP 50052，传输模型张量和计算请求
    v
rpc-worker: rpc-server -> CPU 或 CUDA
```

`rpc-worker` 不需要单独下载 GGUF。模型位于 `main-host`，首次 RPC 加载时会通过
网络发送所需张量；worker 使用 `-c` 后会把张量缓存到本地。

> `llama.cpp` RPC 仍是实验功能，没有认证和传输加密。只在可信局域网中使用，
> 不要将端口 50052 暴露到公网。

## 一、实验前确认

### 1.1 两台机器连接同一网络

优先顺序：

1. 有线局域网。
2. 稳定的 5 GHz/6 GHz Wi-Fi。
3. 手机热点只用于功能验证，不适合稳定性能对比。

记录网络类型，后续报告需要解释 RTT、带宽和抖动。

### 1.2 固定两端版本

在 `main-host` 仓库根目录执行：

```bash
export MAIN_REPO_COMMIT="$(git rev-parse HEAD)"
export LLAMA_COMMIT="$(git -C lab4/third_party/llama.cpp rev-parse HEAD)"
printf 'repo=%s\nllama.cpp=%s\n' "$MAIN_REPO_COMMIT" "$LLAMA_COMMIT"
```

当前 `llama.cpp` 固定 commit 为：

```text
c4a278d68efa17811006f2123a84081dac03fac7
```

在 `rpc-worker` 的 WSL2 Ubuntu 或原生 Ubuntu 中拉取仓库：

```bash
git clone --recurse-submodules https://github.com/114August514/DiPECS.git
cd DiPECS

# 填入 main-host 上一步输出的完整 commit。
export MAIN_REPO_COMMIT="<MAIN_REPO_COMMIT>"
export LLAMA_COMMIT="c4a278d68efa17811006f2123a84081dac03fac7"

git fetch origin
git switch --detach "$MAIN_REPO_COMMIT"
git submodule update --init --recursive

test "$(git rev-parse HEAD)" = "$MAIN_REPO_COMMIT"
test "$(git -C lab4/third_party/llama.cpp rev-parse HEAD)" = "$LLAMA_COMMIT"
```

两台机器输出的仓库 commit 和 submodule commit 必须分别一致。若 worker 已经有
仓库：

```bash
git fetch origin
git switch --detach "$MAIN_REPO_COMMIT"
git submodule update --init --recursive
```

### 1.3 主机模型

后续 RPC 实验使用 Qwen3.5-2B Q4_K_M。当前 GGUF 是社区量化版本：

```bash
mkdir -p lab4/data/models

wget --show-progress -c \
  "https://huggingface.co/bartowski/Qwen_Qwen3.5-2B-GGUF/resolve/main/Qwen_Qwen3.5-2B-Q4_K_M.gguf?download=true" \
  -O lab4/data/models/qwen3.5-2b-q4_k_m.gguf
```

下载后检查：

```bash
export MODEL_PATH="$PWD/lab4/data/models/qwen3.5-2b-q4_k_m.gguf"
stat -c '%s bytes' "$MODEL_PATH"
sha256sum "$MODEL_PATH"
```

当前文件应为：

```text
size:   1396198496 bytes
sha256: 57a1085840f497d764a7fc5d346922dbde961efb54cc792ea81d694fd846a1d8
```

GGUF 只保存在 `main-host`，由 `.gitignore` 排除。

## 二、准备 rpc-worker

以下命令均在 worker 的 WSL2 Ubuntu 或原生 Ubuntu 中执行。

### 2.1 安装公共依赖

```bash
sudo apt update
sudo apt install -y build-essential clang cmake git libcurl4-openssl-dev

clang --version
cmake --version
```

设置路径：

```bash
cd ~/DiPECS
export REPO_ROOT="$PWD"
export RPC_PORT="50052"
export WORKER_CPU_BUILD_DIR="$REPO_ROOT/lab4/third_party/llama.cpp/build-rpc-cpu"
export WORKER_CUDA_BUILD_DIR="$REPO_ROOT/lab4/third_party/llama.cpp/build-rpc-cuda"
```

## 三、CPU worker

没有可用 NVIDIA GPU 时使用本节。

### 3.1 构建 CPU + RPC

```bash
CC=clang CXX=clang++ cmake \
  -S "$REPO_ROOT/lab4/third_party/llama.cpp" \
  -B "$WORKER_CPU_BUILD_DIR" \
  -DCMAKE_BUILD_TYPE=Release \
  -DGGML_RPC=ON

cmake --build "$WORKER_CPU_BUILD_DIR" \
  --config Release \
  --target rpc-server \
  -j "$(nproc)"
```

验证：

```bash
"$WORKER_CPU_BUILD_DIR/bin/rpc-server" --help
```

### 3.2 启动 CPU worker

```bash
"$WORKER_CPU_BUILD_DIR/bin/rpc-server" \
  --host 0.0.0.0 \
  --port "$RPC_PORT" \
  --threads "$(nproc)" \
  --cache
```

保持终端运行。日志中应出现 endpoint 和 `CPU` 设备。

另开一个 WSL/Ubuntu 终端确认监听：

```bash
ss -ltn | grep ":${RPC_PORT}"
```

## 四、NVIDIA CUDA worker

只有当 WSL2 Ubuntu 内能够访问 NVIDIA GPU 时才使用本节。

### 4.1 WSL2 GPU 前置检查

Windows 侧安装支持 WSL 的 NVIDIA 驱动。不要在 WSL2 中安装 Linux 内核驱动。

在 WSL2 Ubuntu 中执行：

```bash
nvidia-smi
nvcc --version
```

如果 `nvidia-smi` 可用但 `nvcc` 不存在，需要按照 NVIDIA 的 CUDA on WSL 文档
安装 WSL-Ubuntu CUDA Toolkit。应安装 toolkit，不要安装会携带 Linux 驱动的
`cuda` 或 `cuda-drivers` 元包。

### 4.2 构建 CUDA + RPC

CPU 与 CUDA 使用不同的构建目录，避免 CMake cache 混用：

```bash
cmake \
  -S "$REPO_ROOT/lab4/third_party/llama.cpp" \
  -B "$WORKER_CUDA_BUILD_DIR" \
  -DCMAKE_BUILD_TYPE=Release \
  -DGGML_CUDA=ON \
  -DGGML_RPC=ON

cmake --build "$WORKER_CUDA_BUILD_DIR" \
  --config Release \
  --target rpc-server \
  -j "$(nproc)"
```

### 4.3 启动 CUDA worker

先让服务自动暴露可用加速设备：

```bash
"$WORKER_CUDA_BUILD_DIR/bin/rpc-server" \
  --host 0.0.0.0 \
  --port "$RPC_PORT" \
  --cache
```

日志应列出 `CUDA0` 和 GPU 名称。多 GPU 机器需要限制设备时：

```bash
"$WORKER_CUDA_BUILD_DIR/bin/rpc-server" \
  --host 0.0.0.0 \
  --port "$RPC_PORT" \
  --device CUDA0 \
  --cache
```

不要同时启动 CPU worker 和 CUDA worker 占用同一个端口。切换后端时先按
`Ctrl+C` 停止当前服务。

## 五、配置 WSL2 网络

先在 Windows PowerShell 执行：

```powershell
wsl --list --verbose
```

确认 Ubuntu 使用 WSL 2。

后续命令中的发行版名称 `Ubuntu` 必须与 `wsl --list --verbose` 输出完全一致。

### 5.1 推荐：Windows 11 镜像网络

Windows 11 22H2 及以上可在
`$env:USERPROFILE\.wslconfig` 中配置：

```ini
[wsl2]
networkingMode=mirrored
```

保存后在 PowerShell 执行：

```powershell
wsl --shutdown
```

重新进入 Ubuntu 并启动 `rpc-server`。然后以管理员身份打开 PowerShell，允许
Hyper-V 防火墙入站 TCP 50052：

```powershell
New-NetFirewallHyperVRule `
  -Name "LlamaCppRpc50052" `
  -DisplayName "llama.cpp RPC 50052" `
  -Direction Inbound `
  -VMCreatorId "{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}" `
  -Protocol TCP `
  -LocalPorts 50052
```

查询 Windows 局域网 IPv4：

```powershell
Get-NetIPAddress -AddressFamily IPv4 |
  Where-Object {
    $_.InterfaceAlias -notmatch "Loopback|vEthernet" -and
    $_.IPAddress -notlike "169.254.*"
  } |
  Format-Table InterfaceAlias, IPAddress
```

选择当前有线或 Wi-Fi 网卡地址，供 `main-host` 作为 `WORKER_IP`。

### 5.2 兼容方案：WSL2 默认 NAT

NAT 模式下，另一台局域网机器通常不能直接连接 WSL 虚拟机 IP。需要在 Windows
上建立端口转发。

在管理员 PowerShell 中取得 WSL IP：

```powershell
$WslIp = ((wsl.exe -d Ubuntu hostname -I).Trim() -split "\s+")[0]
$WslIp
```

查询 Windows 局域网 IPv4，并把实际地址写入变量：

```powershell
$WindowsLanIp = "192.168.1.20"
```

建立 Windows 到 WSL 的 TCP 转发：

```powershell
netsh interface portproxy add v4tov4 `
  listenaddress=$WindowsLanIp `
  listenport=50052 `
  connectaddress=$WslIp `
  connectport=50052

New-NetFirewallRule `
  -DisplayName "llama.cpp RPC 50052" `
  -Direction Inbound `
  -Action Allow `
  -Protocol TCP `
  -LocalPort 50052 `
  -Profile Private
```

检查规则：

```powershell
netsh interface portproxy show all
Get-NetFirewallRule -DisplayName "llama.cpp RPC 50052"
```

`main-host` 应连接 `$WindowsLanIp`，不是 NAT 后面的 `$WslIp`。WSL 重启后
`$WslIp` 可能变化，此时删除旧转发再重新添加：

```powershell
netsh interface portproxy delete v4tov4 `
  listenaddress=$WindowsLanIp `
  listenport=50052
```

### 5.3 原生 Ubuntu

原生 Ubuntu 不需要 Windows 转发。用以下命令取得局域网 IP：

```bash
hostname -I
```

如果启用了 UFW，仅允许 `main-host` 的地址访问：

```bash
export MAIN_HOST_IP="192.168.1.10"
sudo ufw status
sudo ufw allow from "$MAIN_HOST_IP" to any port 50052 proto tcp
```

## 六、main-host 连接 worker

回到当前 Linux 主机，在仓库根目录设置变量：

```bash
export WORKER_IP="192.168.1.20"
export RPC_PORT="50052"
export WORKER_ENDPOINT="${WORKER_IP}:${RPC_PORT}"
export MODEL_PATH="$PWD/lab4/data/models/qwen3.5-2b-q4_k_m.gguf"
export MAIN_BUILD_DIR="$PWD/lab4/third_party/llama.cpp/build"
```

`WORKER_IP` 的取值：

- WSL2 镜像网络：Windows 有线或 Wi-Fi 局域网 IPv4。
- WSL2 NAT：配置 `portproxy` 时使用的 `$WindowsLanIp`。
- 原生 Ubuntu：Ubuntu 的局域网 IPv4。

### 6.1 网络检查

```bash
ping -c 5 "$WORKER_IP"
nc -vz "$WORKER_IP" "$RPC_PORT"
```

部分防火墙会禁止 ping，因此 TCP 检查成功即可。

### 6.2 远端设备发现

```bash
"$MAIN_BUILD_DIR/bin/llama-cli" \
  --rpc "$WORKER_ENDPOINT" \
  --list-devices
```

输出中必须出现带有 RPC endpoint 的远端 `CPU` 或 `CUDA0`。如果只看到本地设备，
不要继续正式实验，先检查第十节。

## 七、RPC smoke

先运行一次短请求：

```bash
"$MAIN_BUILD_DIR/bin/llama-cli" \
  --model "$MODEL_PATH" \
  --prompt "用中文简要说明 RPC 推理为什么可能比单机更慢。" \
  --n-predict 64 \
  --threads 12 \
  --ctx-size 1024 \
  --batch-size 64 \
  --seed 42 \
  --temp 0.7 \
  --reasoning off \
  --reasoning-budget 0 \
  --n-gpu-layers all \
  --rpc "$WORKER_ENDPOINT" \
  --single-turn \
  --no-display-prompt \
  --simple-io \
  --show-timings \
  --color off
```

成功标准：

- `main-host` 退出码为 0。
- worker 终端出现连接、张量或计算日志。
- 输出包含回答和 Prompt/Generation timing。

首次运行可能包含约 1.4 GB 模型张量传输，明显慢于后续运行属于正常现象。完成
smoke 后保留 worker 进程和缓存，再执行正式性能测试。

## 八、单机与 RPC 对照实验

### 8.1 控制变量

两组必须固定：

- Qwen3.5-2B Q4_K_M 模型及 SHA-256。
- `threads=12`。
- `ctx-size=1024`。
- `batch-size=64`。
- `seed=42`。
- `temperature=0.7`。
- 禁用 reasoning，避免思考长度差异干扰性能。
- 相同 prompt 和最大生成长度。
- 每组至少重复 3 次。

单机组使用本地 CPU；RPC 组使用 `--n-gpu-layers all` 将模型层放到远端设备。

### 8.2 llama-bench 单机组

```bash
"$MAIN_BUILD_DIR/bin/llama-bench" \
  --model "$MODEL_PATH" \
  --threads 12 \
  --batch-size 64 \
  --n-prompt 128 \
  --n-gen 64 \
  --repetitions 3 \
  --n-gpu-layers 0 \
  --output jsonl \
  > lab4/data/results/rpc-single-bench-qwen35.jsonl
```

### 8.3 llama-bench RPC 组

```bash
"$MAIN_BUILD_DIR/bin/llama-bench" \
  --model "$MODEL_PATH" \
  --threads 12 \
  --batch-size 64 \
  --n-prompt 128 \
  --n-gen 64 \
  --repetitions 3 \
  --n-gpu-layers 99 \
  --rpc "$WORKER_ENDPOINT" \
  --output jsonl \
  > lab4/data/results/rpc-distributed-bench-qwen35.jsonl
```

### 8.4 Rust 单机质量组

```bash
cargo run --manifest-path lab4/Cargo.toml \
  -p lab4-tools \
  --bin lab4-bench -- \
  --prompts lab4/data/prompts/quality-prompts.jsonl \
  --executable "$MAIN_BUILD_DIR/bin/llama-cli" \
  --model "$MODEL_PATH" \
  --output lab4/data/results/rpc-single-quality-qwen35.jsonl \
  --mode rpc-single-qwen35 \
  --case-prefix rpc-single-qwen35 \
  --repetitions 3 \
  --arg=--threads --arg=12 \
  --arg=--ctx-size --arg=1024 \
  --arg=--batch-size --arg=64 \
  --arg=--seed --arg=42 \
  --arg=--temp --arg=0.7 \
  --arg=--reasoning --arg=off \
  --arg=--reasoning-budget --arg=0 \
  --arg=--n-gpu-layers --arg=0 \
  --arg=--single-turn \
  --arg=--no-display-prompt \
  --arg=--simple-io \
  --arg=--show-timings \
  --arg=--color --arg=off
```

### 8.5 Rust RPC 质量组

```bash
cargo run --manifest-path lab4/Cargo.toml \
  -p lab4-tools \
  --bin lab4-bench -- \
  --prompts lab4/data/prompts/quality-prompts.jsonl \
  --executable "$MAIN_BUILD_DIR/bin/llama-cli" \
  --model "$MODEL_PATH" \
  --output lab4/data/results/rpc-distributed-quality-qwen35.jsonl \
  --mode rpc-distributed-qwen35 \
  --case-prefix rpc-distributed-qwen35 \
  --repetitions 3 \
  --rpc "$WORKER_ENDPOINT" \
  --arg=--threads --arg=12 \
  --arg=--ctx-size --arg=1024 \
  --arg=--batch-size --arg=64 \
  --arg=--seed --arg=42 \
  --arg=--temp --arg=0.7 \
  --arg=--reasoning --arg=off \
  --arg=--reasoning-budget --arg=0 \
  --arg=--n-gpu-layers --arg=all \
  --arg=--single-turn \
  --arg=--no-display-prompt \
  --arg=--simple-io \
  --arg=--show-timings \
  --arg=--color --arg=off
```

汇总：

```bash
cargo run --manifest-path lab4/Cargo.toml \
  -p lab4-tools \
  --bin lab4-summarize -- \
  lab4/data/results/rpc-single-quality-qwen35.jsonl

cargo run --manifest-path lab4/Cargo.toml \
  -p lab4-tools \
  --bin lab4-summarize -- \
  lab4/data/results/rpc-distributed-quality-qwen35.jsonl
```

## 九、实验记录清单

至少保存以下证据：

- 两台机器的仓库 commit 与 `llama.cpp` commit。
- 两台机器的 CPU、内存、GPU、OS 和内核。
- WSL 版本以及使用 NAT 还是 mirrored networking。
- 有线、Wi-Fi 或热点网络类型。
- `ping` 的平均 RTT；若 ping 被禁，记录该情况。
- `rpc-worker` 启动日志和远端设备列表。
- 一次成功 RPC 推理终端截图。
- 单机与 RPC 的 JSONL 原始数据。
- 模型文件名、大小和 SHA-256。
- 首次模型传输与缓存后运行的差异。

报告不要只写“RPC 更快”或“RPC 更慢”，需要结合以下因素解释：

- TCP 往返延迟和网络带宽。
- 模型张量首次传输。
- CPU/GPU 计算能力差异。
- 主机与 worker 的同步等待。
- 模型切分和 KV cache 放置。
- Wi-Fi 抖动和后台任务。

## 十、故障排查

### 10.1 `nc` 显示 Connection refused

依次检查：

```bash
# rpc-worker
ss -ltn | grep ':50052'

# main-host
nc -vz "$WORKER_IP" "$RPC_PORT"
```

确认 `rpc-server` 使用 `--host 0.0.0.0`，并检查 Windows/Linux 防火墙。

### 10.2 WSL2 NAT 下局域网无法连接

确认 `main-host` 使用的是 Windows 局域网 IP，不是 WSL NAT IP。重新执行：

```powershell
netsh interface portproxy show all
wsl.exe -d Ubuntu hostname -I
```

WSL IP 改变后重建 `portproxy`。

### 10.3 能连端口但发现不到 RPC 设备

检查两端 submodule commit：

```bash
git -C lab4/third_party/llama.cpp rev-parse HEAD
```

两端必须一致，并且均以 `-DGGML_RPC=ON` 构建。

### 10.4 CUDA worker 只显示 CPU

在 WSL2 执行：

```bash
nvidia-smi
nvcc --version
```

然后确认 CUDA 构建命令包含 `-DGGML_CUDA=ON`，并使用
`build-rpc-cuda/bin/rpc-server`，不是 CPU 构建目录。

### 10.5 首次运行长时间无输出

Qwen3.5-2B Q4_K_M 约 1.4 GB。首次运行需要传输模型张量，Wi-Fi 下可能等待较久。
观察 worker 日志和网络流量，不要立即终止。后续运行应受益于 `--cache`。

### 10.6 开启详细日志

只在排错时使用，正式测量时关闭：

```bash
GGML_RPC_DEBUG=1 \
  "$WORKER_CPU_BUILD_DIR/bin/rpc-server" \
  --host 0.0.0.0 \
  --port "$RPC_PORT" \
  --threads "$(nproc)" \
  --cache
```

## 十一、停止和清理

在 worker 终端按 `Ctrl+C` 停止 `rpc-server`。

若 WSL2 NAT 实验结束后不再需要端口转发，在管理员 PowerShell 中执行：

```powershell
netsh interface portproxy delete v4tov4 `
  listenaddress=$WindowsLanIp `
  listenport=50052

Remove-NetFirewallRule -DisplayName "llama.cpp RPC 50052"
```

镜像网络的 Hyper-V 防火墙规则可用以下命令删除：

```powershell
Remove-NetFirewallHyperVRule -Name "LlamaCppRpc50052"
```

## 参考资料

- [llama.cpp RPC](https://github.com/ggml-org/llama.cpp/blob/c4a278d68efa17811006f2123a84081dac03fac7/tools/rpc/README.md)
- [Microsoft WSL 网络](https://learn.microsoft.com/windows/wsl/networking)
- [Microsoft WSL 基础命令](https://learn.microsoft.com/windows/wsl/basic-commands)
- [NVIDIA CUDA on WSL](https://docs.nvidia.com/cuda/wsl-user-guide/)
- [Qwen3.5-2B Q4_K_M GGUF](https://huggingface.co/bartowski/Qwen_Qwen3.5-2B-GGUF/blob/main/Qwen_Qwen3.5-2B-Q4_K_M.gguf)
