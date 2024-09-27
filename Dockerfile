# 使用官方 Rust 镜像作为构建环境
FROM rust:1.70 as builder

# 设置工作目录
WORKDIR /usr/src/rust-redis-clone

# 复制整个项目
COPY . .

# 构建项目
RUN cargo build --release

# 使用一个更小的基础镜像来运行应用
FROM debian:buster-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /usr/src/rust-redis-clone/target/release/rust-redis-clone /usr/local/bin/rust-redis-clone

# 设置工作目录
WORKDIR /app

# 创建数据目录
RUN mkdir -p /app/data

# 暴露应用端口
EXPOSE 6379

# 运行应用
CMD ["rust-redis-clone"]