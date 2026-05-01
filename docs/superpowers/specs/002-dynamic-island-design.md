# Dynamic Island Status Indicator Design

## Overview

A Dynamic Island-style status indicator for cc-helper, inspired by Apple's notch-area UI. Shows as a compact pill on screen, auto-expands on events (permission requests, questions), supports dragging to reposition.

## Visual Design

### Shape

Rounded pill (border-radius ~22px), dark background (#1c1c1e), floating above all windows.

### States

1. **Idle** — Small pill, gray dot, "cc-helper idle"
2. **Working** — Green pulsing dot, "N sessions · working"
3. **Permission Request** — Yellow pulsing dot, auto-expands to show Allow/Deny with tool details
4. **AskUserQuestion** — Blue pulsing dot, auto-expands to show question options
5. **Stop/StopFailure** — Brief notification, then returns to idle

### Auto-Expand Behavior

- Permission requests and questions trigger auto-expand
- Expand animation: pill grows downward, content fades in (~200ms)
- User responds (Allow/Deny/Select) → auto-collapse (~300ms delay)
- Click outside or press Escape → collapse

### Dragging

- Default position: top center of screen
- User can drag to any position
- Position persists across app restarts (saved to config)
- While dragging, expanded state collapses

## Platform Adaptations

### macOS (with notch)

- Pill aligns below the notch area, appears integrated
- Respects safe area

### macOS (without notch) / Linux / Windows

- Pill positioned at top center, below system title bar area
- Same behavior, different default vertical offset

## Architecture

### New Tauri Window

A dedicated "island" window separate from the permission dialog:

```
Window: "island"
├── Always on top
├── Transparent background
├── Frameless
├── Skip taskbar
├── Click-through when collapsed (optional)
└── Resizable programmatically (collapsed vs expanded)
```

### Frontend Component

```
DynamicIsland.vue
├── CollapsedView (pill with status dot)
├── ExpandedView
│   ├── PermissionPanel (Allow/Deny for tool requests)
│   ├── QuestionPanel (options for AskUserQuestion)
│   └── SessionList (click to view sessions)
└── DragHandler (reposition)
```

### Data Flow

```
IPC event received
  → DialogManager checks event type
    → If PermissionRequest: show in island (NOT separate dialog)
    → If other event: update island status
  → User interacts with island
    → resolve() sends response via oneshot channel
```

### Key Changes from Current Design

- Permission dialogs merge INTO the island (no separate popup windows)
- Island window is persistent (always visible, state changes)
- Replace DialogManager's popup window creation with island state updates

### State Management

```rust
struct IslandState {
    status: IslandStatus,           // Idle, Working, WaitingPermission, WaitingQuestion
    sessions: Vec<SessionSummary>,  // Active sessions
    pending_request: Option<PendingRequest>,  // Current permission/question
}
```

Tauri events for island updates:
- `island:update-status` — status change
- `island:show-request` — expand with permission/question
- `island:resolve` — user responded, collapse

## Interaction Details

### Permission Request Flow

1. Hook sends PermissionRequest via socket
2. Daemon updates island state → `island:show-request` event
3. Island auto-expands with tool details + Allow/Deny
4. User clicks → `resolve_permission` command → response sent back

### AskUserQuestion Flow

1. Same as above, but island shows question + options
2. User selects option (or types custom) → Submit
3. Response includes `updatedInput` with answers

### Dragging

- Mouse down on pill → start drag
- Mouse move → update window position
- Mouse up → save position to config
- While dragging, expanded content collapses

## Configuration

```json
{
  "island": {
    "position": { "x": null, "y": null },  // null = auto-center
    "collapsed_size": { "width": 160, "height": 36 },
    "expanded_size": { "width": 380, "height": 300 }
  }
}
```

## Implementation Notes

- Use Tauri's `WebviewWindowBuilder` with `transparent(true)` for glass effect
- CSS `backdrop-filter: blur()` for frosted glass behind pill
- Window resize animations via Tauri `set_size` API
- Consider `always_on_top(true)` + `focusable(false)` for non-intrusive overlay
