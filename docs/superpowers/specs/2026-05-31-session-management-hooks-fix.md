# Session 管理与 Hook 清理修复 Spec

日期：2026-05-31
目标版本：1.1.1

## 问题

cc-helper 当前可能展示出额外会话，常见表现是莫名出现以 `tmp` 等临时目录命名的会话。会话计数也可能不准确，因为历史 Idle 会话会和真实活跃会话一起参与计数。

另外，用户本机安装过旧版本 hook 后，`~/.claude/settings.json` 里可能残留旧的 `SessionEnd` pilot-hook 配置。当前协议已经不再解析 `session-end`，所以该 hook 属于无效配置，重新安装 hooks 时应被清理。

## 根因

- `Stop` 和 `SubagentStop` 在找不到已有 `session_id` 时会反向创建 Idle 会话，导致临时子任务或 stop 事件变成可见会话。
- Session 存储只使用 `session_id` 作为 key，但设计语义要求会话身份包含 `session_id` 和 `cwd`。
- `get_sessions()` 直接返回 HashMap values，顺序不稳定。
- Island 和托盘计数使用 retained sessions 总数，包含 Idle 历史会话。
- Hook 安装逻辑只会清理当前事件名下的旧 pilot-hook 命令，不会清理 `SessionEnd` 这种已经废弃的事件名。
- Island 初始状态在 Vue 组件 mount 后才异步读取，首帧状态快照可能丢失。

## 目标行为

- 未知 `Stop` 和 `SubagentStop` 事件不得创建新会话。
- `PermissionRequest` 只有在 `session_id` 和 `cwd` 都有效时，才允许兜底创建会话。
- 会话身份必须区分同一个 `session_id` 下的不同 `cwd`。
- 会话列表排序稳定：等待权限优先，其次 Working、Failed、Idle；同状态内按最近事件优先。
- 托盘和 Island 的数量展示只代表活跃会话，不包含 Idle 历史会话。
- 重新安装 hooks 时，应完整删除遗留的废弃 `SessionEnd` 配置。
- Island 启动时应先读取初始状态，再挂载 Vue 组件。

## 实施范围

- 调整 `SessionManager` 的状态流转、session key、排序和活跃会话计数。
- 托盘和 Island payload 使用活跃会话数。
- 同步更新 Tauri 内置安装器和独立 `cc-helper-hook install`，清理废弃 hook 事件。
- 调整 Island 初始化顺序，避免初始状态丢失。
- 增加单元测试覆盖未知 stop、cwd 感知 key、活跃计数、稳定排序。

## 验证项

- 执行 `cargo test`。
- 执行 `npm run build`。
- 重新执行 `cc-helper-hook install`。
- 确认 `~/.claude/settings.json` 不再包含有效的 `SessionEnd` pilot-hook 配置。