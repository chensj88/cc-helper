# Changelog

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