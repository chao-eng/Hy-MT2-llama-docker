# Hy-MT2-llama-docker

> 🐳 小型轻量级腾讯混元 Hy-MT2 翻译模型的 Docker 容器化部署方案，基于 llama.cpp 推理引擎，提供开箱即用、超低资源占用的 OpenAI 兼容翻译 API 服务。

## 📖 项目简介

[Hy-MT2](https://huggingface.co/collections/tencent/hy-mt2) 是腾讯混元团队开源的新一代多语言翻译模型，支持 **33 种语言**互译，在多项评测中超越主流商业翻译 API。

本项目致力于提供**小型轻量级**的容器化部署方案，特别针对 `Hy-MT2-1.8B` 极限量化版本进行深度适配与优化。通过 **多阶段 Docker 构建**，将 llama.cpp 的特定 PR 分支（支持 STQ 1.25-bit / 2-bit 极限量化）编译为高性能推理服务，实现：

- 🚀 **纯 CPU 推理**，无需 GPU，降低部署门槛
- 📦 **极致精简镜像**，多阶段构建，仅保留运行时必要组件
- ⚡ **CPU 多架构优化**，开启 `-O3` 结合 OpenMP 多线程并行加速，兼顾性能与兼容性
- 🔌 **OpenAI 兼容 API**，可无缝对接各类翻译工具链

## 🏗️ 模型系列

| 模型 | 参数量 | 适用场景 | 说明 |
|------|--------|----------|------|
| **Hy-MT2-1.8B** | 1.8B | 端侧 / 轻量部署 | 极限量化版本极小（1.25-bit: 462MB, 2-bit: 600MB, 4-bit: 1.13GB） |
| **Hy-MT2-7B** | 7B | 通用翻译 | 质量与性能均衡 |
| **Hy-MT2-30B-A3B** | 30B (MoE) | 专业领域 | 混合专家架构，复杂文本翻译 |

## 📋 前提条件

- Docker 已安装（建议 20.10+）
- 足够的磁盘空间（构建时需要编译 llama.cpp）
- 已下载 GGUF 格式的 Hy-MT2 模型文件

### 获取模型

推荐从 ModelScope 或 Hugging Face 下载所需的 GGUF 模型文件（以下以 1.8B 模型为例）：

- **1.25Bit 极限量化版本** (约 462MB)：
  - ModelScope 下载地址：[Hy-MT2-1.8B-1.25Bit-GGUF](https://modelscope.cn/models/Tencent-Hunyuan/Hy-MT2-1.8B-1.25Bit-GGUF)
- **2Bit 极限量化版本** (约 600MB)：
  - ModelScope 下载地址：[Hy-MT2-1.8B-2Bit-GGUF](https://modelscope.cn/models/Tencent-Hunyuan/Hy-MT2-1.8B-2Bit-GGUF)
- **常规量化版本（如 4Bit `Q4_K_M``）** (约 1.13GB)：
  - ModelScope 下载地址：[Hy-MT2-1.8B-GGUF](https://modelscope.cn/models/Tencent-Hunyuan/Hy-MT2-1.8B-GGUF)
- **Hugging Face 官方合集**：
  - [Tencent Hy-MT2 官方空间](https://huggingface.co/collections/tencent/hy-mt2)

## 🚀 快速开始

<details open>
<summary><b>🐳 使用作者打包的镜像 (推荐，无需本地编译)</b></summary>
<br>

##### 1. 运行容器 (默认推荐镜像，针对 1.25-bit 及 4-bit量化 ，使用 PR #22836)

```bash
docker run -d \
  --name hy-mt2 \
  -p 8080:8080 \
  -v /path/to/your/models:/models \
  crpi-a1liy20beodq2bdl.cn-beijing.personal.cr.aliyuncs.com/bujic/llama-server:pr22836 \
  -m /models/your-model.gguf \
  -c 4096 \
  -t $(nproc)
```

##### 2. Docker Compose 部署 (把模型文件放在 `./models` 目录下, 创建 `docker-compose.yml`)
```yaml
services:
  llama-server:
    image: crpi-a1liy20beodq2bdl.cn-beijing.personal.cr.aliyuncs.com/bujic/llama-server:pr22836 #镜像包
    container_name: llama-server
    restart: unless-stopped
    volumes:
      - ./models:/models #模型目录
    ports:
      - "8080:8080"
    command: >
      -m /models/Hy-MT2-1.8B-1.25Bit.gguf
      -c 2048 
      -t 8
      --parallel 2
      --cont-batching
```

```bash
docker compose up -d
```

> **💡 镜像与模型文件对应说明**
> 
> | 目标模型文件 | 使用的镜像名称 (无需本地构建) |
> | `Hy-MT2-1.8B-2Bit-GGUF` | `crpi-a1liy20beodq2bdl.cn-beijing.personal.cr.aliyuncs.com/bujic/llama-server:pr19357` |
> 
> **📊 性能与选择建议**
> 
> 经过实测，两者的推理速度差异较大：
> - **PR #22836 镜像（1.25Bit / 4Bit）**：推理速度达到 **28.94 tokens/s**
> - **PR #19357 镜像（2Bit）**：推理速度仅为 **7.19 tokens/s**
> 
> 💡 **选择建议**：强烈建议优先使用 **PR #22836** 版本镜像配合 **1.25Bit 极限量化模型**（`Hy-MT2-1.8B-1.25Bit-GGUF`）。它的推理速度是 2Bit 版本的 **3 倍左右**，且模型体积更小（仅约 462MB）。

</details>

<details>
<summary><b>🛠️ 手动编译构建部署 (适合自定义修改)</b></summary>
<br>

##### 1. 构建镜像

```bash
# 默认构建（针对 1.25-bit 及常规量化，使用 PR #22836）
docker build -t hy-mt2-server .

# 构建支持 2-bit 的版本（使用 PR #19357）
docker build --build-arg PR_NUM=19357 -t hy-mt2-server:2bit .
```

##### 2. 运行容器

```bash
docker run -d \
  --name hy-mt2 \
  -p 8080:8080 \
  -v /path/to/your/models:/models \
  hy-mt2-server \
  -m /models/your-model.gguf \
  -c 4096 \
  -t $(nproc)
```

> **💡 构建参数与模型文件对应说明**
> 
> 在执行 `docker build` 时，必须通过 `--build-arg PR_NUM=xxxx` 来编译支持特定模型的 llama.cpp 版本。对应关系如下：
> 
> | 目标模型文件 | 编译所需 `PR_NUM` 参数 | 构建命令示例 |
> |--------------|----------------------|--------------|
> | `Hy-MT2-1.8B-1.25Bit-GGUF` 或 `Hy-MT2-1.8B-Q4_K_M.gguf` | `22836` (默认值) | `docker build -t hy-mt2-server .` |
> | `Hy-MT2-1.8B-2Bit-GGUF` | `19357` | `docker build --build-arg PR_NUM=19357 -t hy-mt2-server:2bit .` |

</details>

<br>

**参数说明：**

| 参数 | 说明 |
|------|------|
| `-m /models/your-model.gguf` | 模型文件路径（容器内路径） |
| `-c 4096` | 上下文长度 |
| `-t $(nproc)` | 推理线程数，建议设置为 CPU 核心数 |
| `-tb 4` | 端口映射 |
| `-p 8080:8080` | 批处理线程数 |
| `-b 512` | 批处理大小 |
| `--parallel 2` | 并发请求数 |
| `--cont-batching` | 启用连续批处理 |
### llama-server 常用参数



### 💾 内存开销与性能评估 (-c 4096)

在 `llama.cpp` 中，大模型运行时的实际总物理内存占用计算公式为：
$$\text{总内存} = \text{模型权重体积 (Model Weights)} + \text{上下文缓存体积 (KV Cache)} + \text{服务框架开销与冗余}$$

对于 **Hy-MT2-1.8B** 模型，当把 `-c` (上下文长度) 设置为 **4096** 时，不同量化版本的实际内存占用与性能影响分析如下：

#### 1. 1.25-bit 极限量化版本
*   **💾 模型权重**：约 **430 MiB** (1.8B 参数经 STQ 极限量化压缩)
*   **⚡ KV Cache 占用**：当 `-c` 为 512 时仅约 16 MiB；但拉大到 **4096** 时将线性暴涨至约 **128 MiB - 140 MiB**。
*   **📊 总内存估计**：模型权重 (430 MiB) + KV Cache (135 MiB) + llama-server 框架开销 (约 50 MiB) + 多并发/OpenMP 冗余 (约 100 MiB) ≈ **650 MiB - 700 MiB**。
*   **⚠️ 性能影响**：
    *   **首字延迟 (Prompt Eval)**：处理超长文本或长视频字幕时，纯 CPU 计算“预填 (Prefill)”需要一定时间。
    *   **推理速度变化**：长上下文多核频繁交换数据会增加内存带宽压力，生成速度可能会从空上下文的 21 tokens/s 稍下滑至 **15 tokens/s** 左右。
    *   **📌 建议**：常规电影/短视频字幕处理建议使用 `-c 2048` 甜点位，性能更佳；如有整本小说等长文本需求，开到 4096 内存也完全足够。

---

#### 2. 4-bit 常规量化版本 (如 Q4_K_M)
*   **💾 模型权重**：约 **1.1 GiB**。
*   **⚡ KV Cache 占用**：在运行 4bit 模型时，为了保证长上下文翻译质量不坍塌，`llama.cpp` 默认会使用 **FP16/BF16 高精度** 存储 KV Cache，在 4096 长度下约占用 **260 MiB - 280 MiB**。
*   **📊 总内存估计**：模型权重 (1.1 GiB) + KV Cache (280 MiB) + 框架开销与冗余 (约 200 MiB) ≈ **1.5 GB - 1.6 GB**。

---

### 3. 验证与访问

*   **API 健康检查**：
    ```bash
    curl http://localhost:8080/health
    ```
*   **🎈 内置 Web 交互界面**：
    `llama-server` 默认打包内置了一个非常轻量且全功能的 **Chat 网页客户端**。
    容器启动成功后，无需安装任何额外客户端，直接在浏览器中打开：
    ```
    http://localhost:8080/
    ```
    即可访问。您可以在该界面与 Hy-MT2 翻译模型进行可视化对话、测试翻译效果、动态调整温度参数，甚至能够直接上传 text/PDF 文本文件作为翻译上下文。

## 📡 API 使用

服务启动后提供 **OpenAI 兼容的 Chat Completions API**。

### 翻译请求与响应示例

#### 1. 翻译请求 (cURL)

```bash
curl http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [
        {
            "role": "user",
            "content": "将以下文本翻译为 中文。注意只需要输出翻译后的结果，不要额外解释：Simplicity is the soul of efficiency. The best code is not the one that spans thousands of lines, but the one that solves a complex problem with elegant minimalism."
        }
    ],
    "temperature": 0.7,
    "top_p": 0.6,
    "top_k": 20,
    "repetition_penalty": 1.05,
    "max_tokens": 2048
  }'
```

#### 2. API 响应

```json
{
    "choices": [
        {
            "finish_reason": "stop",
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "简洁是效率的灵魂。最好的代码并非长达数千行的代码，而是以简洁的方式解决复杂问题的代码。"
            }
        }
    ],
    "created": 1781864213,
    "model": "Hy-MT2-1.8B-Q4_K_M.gguf",
    "system_fingerprint": "b9521-f8b355a9e",
    "object": "chat.completion",
    "usage": {
        "completion_tokens": 23,
        "prompt_tokens": 55,
        "total_tokens": 78,
        "prompt_tokens_details": {
            "cached_tokens": 10
        }
    },
    "id": "chatcmpl-UmSvxuAlbfI2ipATcSiZxOCh2w4usmey",
    "timings": {
        "cache_n": 10,
        "prompt_n": 45,
        "prompt_ms": 776.621,
        "prompt_per_token_ms": 17.258244444444443,
        "prompt_per_second": 57.943321130899115,
        "predicted_n": 23,
        "predicted_ms": 1318.048,
        "predicted_per_token_ms": 57.3064347826087,
        "predicted_per_second": 17.45004734273714
    }
}
```

#### 3. 核心推理参数说明

为了在翻译任务中获得最稳定、精确且自然的译文，建议使用上述示例中的参数配置。各参数的详细作用如下：

| 参数名 | 示例值 | 作用说明 |
| :--- | :---: | :--- |
| **`temperature`** | `0.7` | **随机性温度**。控制生成文本的创造力与多样性。对于翻译任务，建议取值在 `0.1`（严格直译、极其稳定）到 `0.7`（适合意译、语言更流畅自然）之间。 |
| **`top_p`** | `0.6` | **核采样阈值**。仅从累积概率达到 60% 的候选词中筛选。设置较低的 `top_p`（如 `0.6`）能有效过滤低概率的非主流词汇，使翻译语法更严谨。 |
| **`top_k`** | `20` | **最高概率词数**。限制每次生成时只从概率最高的前 20 个候选词中进行选择。这可以防止翻译跑题或产生词不对意的情况。 |
| **`repetition_penalty`** | `1.05` | **重复惩罚系数**。对已生成词的重复出现进行微弱惩罚。设置为 `1.05` 可以在不破坏正常重词（如排版、标点）的前提下，有效避免模型陷入无限复读。 |
| **`max_tokens`** | `2048` | **单次最大输出 Token 数**。控制单次 API 请求生成翻译结果的最大长度。该限制不等于模型的最大上下文（`-c`），仅限制输出结果的上限。 |


## 📝 Hy-MT2 官方提示词（Prompt）指南

为了获得最佳的翻译效果，推荐使用官方设计的系统提示词（Prompt）。

> ⚠️ **注意**：在以下示例中，`source_lang` 和 `target_lang` 均应使用完整的语言名称。例如：中文提示中应使用“中文”、“英文”、“日语”等，英文提示中应使用 "Chinese", "English", "Japanese" 等。

### 1. 默认翻译 (Default Translation)

*   **中文提示**：
    ```text
    将以下文本翻译为 {target_lang}，注意只需要输出翻译后的结果，不要额外解释：

    {source_text}
    ```
*   **英文提示**：
    ```text
    Translate the following text into {target_lang}. Note that you should only output the translated result without any additional explanation:

    {source_text}
    ```

---

### 2. 术语翻译 (Translation with Glossary)

*   **中文提示**：
    ```text
    参考下面的翻译：
    {text} 翻译成 {text}
    {text} 翻译成 {text}
    {text} 翻译成 {text}
    将以下文本翻译为 {target_lang}，注意只需要输出翻译后的结果，不要额外解释：

    {source_text}
    ```
*   **英文提示**：
    ```text
    Reference the following translations:
    {text} translates to {text}
    {text} translates to {text}
    {text} translates to {text}

    Translate the following text into {target_lang}. Note that you must ONLY output the translated result without any additional explanation:

    {source_text}
    ```

---

### 3. 风格翻译 (Style-Constrained Translation)

*   **中文提示**：
    ```text
    请将以下文本翻译为 {target_lang}。
    注意翻译的风格要严格符合【{target_style}】

    {source_text}
    ```
*   **英文提示**：
    ```text
    Please translate the following text into {target_lang}. Note that the translation style must strictly conform to [{target_style}]:

    {source_text}
    ```

---

### 4. 个性化偏好翻译 (Personalized Translation)

*   **中文提示**：
    ```text
    【待翻译文本】
    {source_text}

    【翻译任务】
    1、{user_preferences}
    2、{user_preferences}
    3、……
    4、将【待翻译文本】翻译为 {target_lang}。
    ```
*   **英文提示**：
    ```text
    [Source Text]
    {source_text}

    [Translation Tasks]
    1. {user_preferences}
    2. {user_preferences}
    3. ...
    4. Translate the [Source Text] into {target_lang}.
    ```

---

### 5. 带分隔符的翻译 (Translation with Delimiters)

*   **中文提示**：
    ```text
    请将以下文本准确翻译为 {target_lang}。
    你必须在译文中保留等量的分隔符，绝对不可遗漏、转义或翻译该符号，并注意分隔符的位置。

    {source_text}
    ```
*   **英文提示**：
    ```text
    Please accurately translate the following text into {target_lang}.
    You must retain the exact same number of delimiters in the translation. Strictly do not omit, escape, or translate these symbols, and pay close attention to their placement.

    {source_text}
    ```

---

### 6. 结构化数据翻译 1 (Structured Data Translation - Rules)

*   **中文提示**：
    ```text
    # 任务目标
    将下方 {source_text} 中的 {format_type} 格式数据翻译为 {target_lang}。

    # 严格约束
    1. 结构锁定：绝对保持原有的 {format_type} 数据结构、缩进和层级完全不变。
    2. 选择性翻译：仅翻译面向用户展示的可见文本内容。
    3. 禁止修改：严禁翻译或更改任何代码标签、键名 (Key)、变量占位符（如 {{var}}、${var}、%s、%d 等）或代码属性。

    # 数据输入
    {source_text}
    ```
*   **英文提示**：
    ```text
    ### Task
    Translate the user-facing text within the following {format_type} data into {target_lang}.

    ### Strict Rules
    1. Structure Preservation: You MUST preserve the original {format_type} data structure, nesting, hierarchy, and indentation exactly as they are.
    2. Selective Translation: Translate ONLY the visible, user-facing text content/values.
    3. Strict Non-Translation: NEVER translate or alter code tags, keys, properties, object names, or variable placeholders. Leave them exactly in their original English/code form.

    ### Source Data
    {source_text}
    ```

---

### 7. 结构化数据翻译 2 (Structured Data Translation with Background)

*   **中文提示**：
    ```text
    【背景信息】
    {background_text}

    请结合背景信息将以下文本翻译为 {target_lang}。

    【待翻译文本】
    {source_text}
    ```
*   **英文提示**：
    ```text
    [Background Information]
    {background_text}

    Please translate the following text into {target_lang}, taking the provided background information into consideration.

    [Source Text]
    {source_text}
    ```

## 🏗️ 构建原理

本项目采用 **多阶段构建（Multi-stage Build）**，确保最终镜像体积最小：

```
┌─────────────────────────────────────┐
│  Stage 1: Builder                   │
│  ├─ 基础环境: debian:stable-slim    │
│  ├─ 安装编译依赖 (cmake, gcc, git)  │
│  ├─ 克隆 llama.cpp 指定 PR 分支     │
│  └─ 编译 llama-server (开启优化)    │
├─────────────────────────────────────┤
│  Stage 2: Runtime                   │
│  ├─ 基础环境: debian:stable-slim    │
│  ├─ 仅安装 libgomp1 (OpenMP 运行库) │
│  ├─ 复制 llama-server 二进制        │
│  └─ 暴露 8080 端口                  │
└─────────────────────────────────────┘
```

### 关键优化点

1. **多架构兼容与加速**：使用 `-DGGML_NATIVE=OFF` 确保生成的 Docker 镜像在多平台（如 amd64, arm64）上运行时不会因指令集不支持而崩溃，并配合 `-O3` 开启编译器高等级优化
2. **OpenMP 多线程**：`-DGGML_OPENMP=ON` 启用并行计算
3. **最小化依赖**：运行时仅需 `libgomp1`，无编译工具残留
4. **共享库支持**：自动复制并索引 `.so` 动态库

## ⚠️ 注意事项

- **多平台架构支持**：已在 Dockerfile 中通过 `-DGGML_NATIVE=OFF` 移除了特定的本机指令集依赖，确保在 x86_64（如云服务器、Mac Intel）和 arm64（如 Apple Silicon M 系列芯片、树莓派等）架构设备上均可完美运行而不会出现 Illegal instruction 报错。
- **PR 分支依赖**：本项目依赖 llama.cpp 的特定未合并 PR，如 PR 已合并至主线可改为使用 `main` 分支
- **内存需求**：运行时内存需求取决于模型大小和上下文长度，1.8B 模型约需 1-2GB，7B 模型约需 4-8GB

## 📜 许可证

本项目的 Dockerfile 和构建脚本遵循 MIT 协议。

- **Hy-MT2 模型**：遵循腾讯混元开源许可协议，详见 [模型主页](https://huggingface.co/collections/tencent/hy-mt2)
- **llama.cpp**：遵循 MIT 协议，详见 [llama.cpp](https://github.com/ggml-org/llama.cpp)

## 🔗 相关链接

- [Hy-MT2 模型合集 (Hugging Face)](https://huggingface.co/collections/tencent/hy-mt2)
- [llama.cpp](https://github.com/ggml-org/llama.cpp)
- [llama.cpp PR #22836 - STQ 1.25-bit 支持](https://github.com/ggml-org/llama.cpp/pull/22836)
- [llama.cpp PR #19357 - 2-bit 支持](https://github.com/ggml-org/llama.cpp/pull/19357)
