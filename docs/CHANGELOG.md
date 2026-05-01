# 更新日志

## Session 检测修复

### 问题

Helper app 无法检测新的 Claude Code session，hook 事件未到达 Tauri daemon。

### 根因

1. **路径空格问题** — Claude Code 不使用 shell 执行 hook 命令，直接解析命令字符串。`'/path with spaces/hook'` 中的引号被当作字面字符，命令解析失败。
2. **stdin 丢失** — `exec` wrapper 不转发 stdin，Claude Code 通过 stdin 传入的 hook 事件 JSON 未到达 `cc-helper-hook` 二进制。

### 修复

- 创建 `~/.claude/bin/cc-helper-hook` wrapper 脚本，路径无空格
- Wrapper 保存 stdin 到临时文件后重定向给实际二进制：

```bash
#!/bin/bash
stdin_file="/tmp/cc-helper-stdin-$.txt"
cat > "$stdin_file"
'/Applications/cc-helper.app/Contents/MacOS/cc-helper-hook' "$@" < "$stdin_file"
rm -f "$stdin_file"
```

## Session 清理机制完善

### 新增：Staleness 降级

Claude Code 用户退出时不发出任何 hook 事件，session 会卡在 Working/WaitingPermission 状态永远不被清理。

- Working/WaitingPermission 状态超过 **5 分钟**无新事件 → 自动降级为 Idle
- 60 秒周期检查，不依赖新事件触发

### 新增：SubagentStop hook

- 注册 `SubagentStop` hook（子 agent 结束时 Claude Code 发出）
- session.rs 中与 Stop 合并处理，设为 Idle

### 新增：Failed session 清理

- Failed 状态超过 **1 小时**自动删除（此前永不清理）

### 移除：SessionEnd hook

- SessionEnd 不在 Claude Code 实际发出的事件列表中，注册了也不会触发
- 从 hook 注册移除，协议定义保留（向前兼容）