# ==============================================================================
# === 第一阶段：构建环境（通用、参数化、开启指令集加速） ===
# ==============================================================================
FROM debian:stable-slim AS builder

# 1. 锁死基础依赖，清理缓存，减少层污染
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 💡 核心优化一：引入变量控制，默认使用 1.25Bit 对应的 22836
# 编译 2Bit 时只需传入: --build-arg PR_NUM=19357
ARG PR_NUM=22836

# 2. 单层内完成拉取、回滚与编译（极致压榨 CPU 通用矢量指令加速）
RUN git clone https://github.com/ggml-org/llama.cpp.git . && \
    git fetch origin pull/${PR_NUM}/head && \
    git reset --hard FETCH_HEAD && \
    # 💡 核心优化二：针对 Linux CPU 容器开启加速优化
    # -DGGML_OPENMP=ON: 开启多线程并行计算加速
    # -DCMAKE_C_FLAGS / -DCMAKE_CXX_FLAGS: 注入通用底层优化指令（对齐现代 AMD/ARM 架构基本盘）
    cmake -B build \
      -DLLAMA_SERVER=ON \
      -DLLAMA_BUILD_TESTS=OFF \
      -DLLAMA_BUILD_EXAMPLES=OFF \
      -DGGML_OPENMP=ON \
      -DCMAKE_C_FLAGS="-O3 -march=native" \
      -DCMAKE_CXX_FLAGS="-O3 -march=native" && \
    cmake --build build --config Release --target llama-server -j$(nproc)

# ==============================================================================
# === 第二阶段：极致精简的运行时环境 ===
# ==============================================================================
FROM debian:stable-slim

# 💡 核心优化三：根据第一阶段开启的 OpenMP，运行时必须补全 libgomp1 依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# 复制编译产物（按需复制，绝不拖泥带水）
COPY --from=builder /app/build/bin/llama-server /usr/local/bin/llama-server
# 如果该 PR 生成了共享动态库则复制并建立索引，没有也不影响
COPY --from=builder /app/build/bin/*.so* /usr/local/lib/
RUN ldconfig

# 环境变量优化
ENV LLAMA_ARG_HOST=0.0.0.0
ENV LLAMA_ARG_PORT=8080

EXPOSE 8080

# 推荐在 Entrypoint 阶段保持纯净，把具体参数留给 docker run 动态控制
ENTRYPOINT ["llama-server"]