# Changelog

## 1.1.2 - 2026-06-04

### Feature: Stop event notification optimization (merged from pilot)

优化 Stop 事件通知策略，减少桌面通知噪声：

**Changes:**
- Stop 事件不再触发完成通知（避免每轮对话结束弹窗打扰）
- Stop 仍正常更新状态为 idle，保留 Dynamic Island 和托盘状态刷新
- StopFailure 继续触发失败通知（重要的错误提醒保留）
- 添加 `should_notify_for_event` 函数明确通知策略

**Impact:**
- 用户在工作时不再被频繁的任务完成通知打断
- 任务失败时仍能及时收到提醒
- Dynamic Island 和托盘状态切换保持正常

---

## 1.1.0 - 2026-05-06

### Feature: Three-button permission review mode

Added support for the "Always Allow" permission option, matching the Claude Code CLI's three-choice pattern (`Yes` / `Yes, allow all` / `No`).

**Behavior:**
- When `permission_suggestions` is present in the hook payload → shows 3 buttons: **Deny**, **Always Allow**, **Allow**
- When `permission_suggestions` is absent → shows 2 buttons: **Deny**, **Allow**
- Skill mode and other non-suggestion scenarios remain unchanged (2 buttons only)

**Changes across the stack:**

- **island.rs**: Fixed `permission_suggestions` extraction path (was incorrectly nested under `tool_input`, now correctly reads from payload top-level per Claude Code hook protocol). Added `permission_suggestions` field to `PendingRequest`. `resolve()` uses `pending.permission_suggestions` directly.
- **dialog.rs**: Same extraction path fix and `PendingRequest` field addition. Added `suggestions` URL parameter for dialog popup mode.
- **dialog-main.ts**: Extract `permissionSuggestions` from URL params instead of incorrectly reading from `toolInput`.
- **global.d.ts**: Added `always` parameter to `__helperResolve` signature.
- **PermissionDialog.vue**: Added `permissionSuggestions` prop, `allowAlways()` function, conditional "Always Allow" button in all permission modes.
- **DynamicIsland.vue**: Added `permissionSuggestions` to `PendingReq`, `allowAlways()` function, `always` flag in `doAllow()`, conditional "Always Allow" button in all permission modes.
- **styles.css**: Added `.btn.always-allow` styles (light + dark mode).