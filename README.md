# Rust Shell

一个用 Rust 实现的轻量命令行 shell，基于 CodeCrafters 的 Shell 练习项目扩展而来，并经过结构化重构，提升了可维护性和安全性。

## 功能概览

目前实现了以下核心能力：

- 外部命令执行
- PATH 查找与可执行文件识别
- 内建命令：`cd`, `pwd`, `echo`, `type`, `jobs`, `history`, `declare`, `complete`, `exit`
- 管道：`|`
- 重定向：`>`, `>>`, `2>`, `2>>`
- 后台任务：`&`
- 简单变量支持与参数展开
- 历史记录与 readline 交互
- 命令补全接口

## 项目结构

- [src/main.rs](src/main.rs): 程序入口
- [src/shell.rs](src/shell.rs): shell 主循环、输入处理、管道和任务调度
- [src/builtin.rs](src/builtin.rs): 内建命令实现与分发
- [src/program.rs](src/program.rs): 外部程序执行与重定向创建
- [src/context.rs](src/context.rs): 任务上下文、变量管理、后台任务状态
- [src/shell_parse.rs](src/shell_parse.rs): 参数解析、管道和重定向拆分
- [src/shellhelper.rs](src/shellhelper.rs): readline 补全助手

## 运行方式

### 本地开发

```bash
cargo run
```

### 直接运行已编译程序

```bash
./your_program.sh
```

### 运行测试和构建验证

```bash
cargo test --quiet && cargo build --quiet
```

## 设计说明

这个项目的重构重点是把职责分离：

- `shell` 负责用户交互和主循环
- `builtin` 负责 shell 内建命令
- `program` 负责系统命令执行
- `context` 负责变量、任务和状态管理
- `shell_parse` 负责把文本命令解析成可执行结构

这样可以更容易扩展新命令、修复解析边界问题，并减少混合逻辑导致的错误。

## 目前已处理的工程性问题

- 去掉了直接 `panic` 和不安全的 `unwrap` 关键点
- 修正了非法重定向与无效历史参数的处理路径
- 统一了命令处理函数，减少分支耦合
- 增加了回归测试，确保边界情况可控

## 示例

```bash
echo hello
pwd
cd /tmp
ls -l | grep rust
history
declare name=demo
echo $name
```

## 说明

这是一个学习型和练习型 shell 实现，重点在于理解 shell 的执行模型、输入解析和进程管理，而不是完整复制 bash 的所有行为。
