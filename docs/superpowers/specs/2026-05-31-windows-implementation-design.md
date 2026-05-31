# cc-helper Windows 实现设计文档

**日期**: 2026-05-31
**状态**: 设计阶段

## 目标

将 cc-helper 移植到 Windows 平台，保持与 macOS/Linux 版本功能一致，同时适配 Windows 特定的系统 API 和约定。

## 核心技术栈调整

| 组件 | macOS/Linux | Windows |
|------|-------------|---------|
| IPC | Unix Domain Socket | **Named Pipes** |
| 点击穿透 | 40ms 轮询 / GTK input shape | **Layered Window + Region** |
| 系统托盘 | Tauri plugin | **Tauri plugin（统一）** |
| 桌面通知 | tauri-plugin-notification | **tauri-plugin-notification（统一）** |
| Wrapper 脚本 | Bash script | **Batch file (.bat)** |
| 安装路径 | `/Applications/cc-helper.app/` | **`%LOCALAPPDATA%\cc-helper\`** |
| 配置路径 | `~/.claude/` | **`%LOCALAPPDATA%\Claude\`** |
| 打包方式 | .app / .deb | **NSIS installer** |
| 开机自启 | 否 | **否（用户手动启动）** |
| 多显示器 | 支持 | **支持拖拽到任意显示器** |

## 架构设计

### 整体架构（Windows 适配）

```
Claude Code ──(hook stdin)──→ cc-helper-hook.exe ──(Named Pipe)──→ cc-helper.exe
                                                                          │
                                                                 Dynamic Island UI
                                                                          │
Claude Code ←─(hook stdout)── cc-helper-hook.exe ←─(pipe)────────────── 用户操作
```

### 组件职责

| 组件 | 职责 | 技术实现 |
|------|------|----------|
| `cc-helper-hook.exe` | Hook 包装器，连接 Claude Code 和主应用 | Rust + Named Pipe 客户端 |
| `cc-helper.exe` | Tauri 主应用，Island UI + 事件分发 | Rust + Tauri 2 + Vue 3 |
| `helper-protocol` | 共享序列化协议定义 | Rust（跨平台） |

## 关键实现细节

### 1. Named Pipe IPC

#### Pipe 命名规范
- **名称**: `cc-helper-pipe`
- **格式**: `\\.\pipe\cc-helper-pipe`

#### 数据格式
保持与 Unix Socket 一致：JSON+newline

```rust
// Request 结构（不变）
struct Request {
    version: u32,
    event: HookEvent,
    session_id: String,
    cwd: String,
    payload: serde_json::Value,
}

// Response 结构（不变）
struct Response {
    allowed: bool,
    hook_specific_output: Option<serde_json::Value>,
}
```

#### 连接超时
- 客户端连接超时：5 秒
- 读写超时：30 秒（防止挂起）

#### 代码位置
- 新增：`src-tauri/src/ipc_windows.rs`（条件编译 `#[cfg(target_os = "windows")]`）
- 修改：`src-tauri/src/ipc.rs`（平台抽象层）

### 2. 点击穿透实现

#### 技术方案
使用 Windows **Layered Window + Region API**

#### 实现原理
```rust
// 设置窗口为 layered window
SetWindowLongPtrW(hwnd, GWL_EXSTYLE, WS_EX_LAYERED);

// 创建仅包含胶囊区域的 Region
let mut region = RegionBuilder::new();
region.add_rect(capsule_rect); // 仅胶囊区域接收鼠标事件
SetWindowRgn(hwnd, region.to_hrgn(), true);
```

#### 状态切换
- **收起状态**: 仅胶囊区域可交互，其他区域穿透
- **展开状态**: 整个窗口可交互

#### 代码位置
- 修改：`src-tauri/src/island.rs`（添加 Windows 分支）

### 3. Batch Wrapper 脚本

#### 文件位置
`%LOCALAPPDATA%\Claude\bin\cc-helper-hook.bat`

#### 实现内容
```bat
@echo off
setlocal enabledelayedexpansion

:: 保存 stdin 到临时文件
set "stdin_file=%TEMP%\cc-helper-stdin-%RANDOM%.txt"
powershell -NoProfile -Command "$input | Out-File -Encoding UTF8 '!stdin_file!'"

:: 调用实际二进制
"%LOCALAPPDATA%\cc-helper\cc-helper-hook.exe" %* < "!stdin_file!"
set exit_code=!errorlevel!

:: 清理临时文件
del "!stdin_file!" 2>nul

exit /b !exit_code!
```

#### 特殊处理
- 使用 PowerShell 处理 stdin（cmd.exe 不支持 stdin 重定向）
- 保留所有参数传递（`%*`）
- 正确传递退出码

#### 代码位置
- 修改：`crates/cc-helper-hook/src/install.rs`（添加 Windows 分支）

### 4. 安装与卸载

#### 安装路径
```
%LOCALAPPDATA%\cc-helper\
├── cc-helper.exe           # 主应用
├── cc-helper-hook.exe      # Hook 包装器
├── cc-helper-Win0.exe      # Tauri 单文件应用
└── resources/              # 资源文件
```

#### Hook 注册路径
```
%LOCALAPPDATA%\Claude\
├── settings.json           # Claude Code 配置
└── bin\
    └── cc-helper-hook.bat   # Wrapper 脚本
```

#### 配置文件修改
```json
{
  "hooks": {
    "PermissionRequest": "cc-helper-hook.bat",
    "PostToolUse": "cc-helper-hook.bat",
    "Notification": "cc-helper-hook.bat",
    "Stop": "cc-helper-hook.bat",
    "StopFailure": "cc-helper-hook.bat",
    "SessionStart": "cc-helper-hook.bat",
    "SubagentStop": "cc-helper-hook.bat"
  }
}
```

#### 代码位置
- 修改：`src-tauri/src/install.rs`（路径适配 Windows）

### 5. 打包与分发

#### NSIS 安装程序配置

**安装步骤**:
1. 解压文件到 `%LOCALAPPDATA%\cc-helper\`
2. 创建桌面快捷方式
3. 创建开始菜单快捷方式（可选）
4. 注册卸载程序

**卸载步骤**:
1. 停止运行中的 cc-helper 进程
2. 卸载 hooks（修改 settings.json）
3. 删除安装目录
4. 删除快捷方式

#### 配置文件
`src-tauri/tauri.conf.json` 修改：
```json
{
  "bundle": {
    "targets": ["nsis"],
    "windows": {
      "wix": null,
      "nsis": {
        "displayLanguageSelector": false,
        "languages": ["SimpChinese", "English"]
      }
    }
  }
}
```

#### 代码位置
- 新增：`src-tauri/nsis/` 目录和相关配置

### 6. 多显示器支持

#### 实现方式
- **初始位置**: 主显示器顶部中央
- **拖拽支持**: 允许用户拖拽到任意显示器
- **位置记忆**: 保存到配置文件 `%LOCALAPPDATA%\cc-helper\config.json`

#### 配置格式
```json
{
  "island_position": {
    "x": 100,
    "y": 0,
    "monitor": "\\\\.\\DISPLAY1"
  }
}
```

#### 代码位置
- 修改：`src-tauri/src/island.rs`（保存/恢复位置）

## 平台差异处理

### 条件编译策略

```rust
// ipc.rs - 平台抽象层
#[cfg(target_os = "windows")]
use ipc_windows::WindowsIpc;

#[cfg(not(target_os = "windows"))]
use ipc_unix::UnixIpc;

// install.rs - 路径抽象
fn get_install_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    return dirs::local_appdata().join("cc-helper");

    #[cfg(not(target_os = "windows"))]
    return PathBuf::from("/Applications/cc-helper.app");
}
```

### 依赖更新

`Cargo.toml` 新增依赖：
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Gdi",
] }
```

## 实现计划

### Phase 1: IPC 适配
- [ ] 实现 Named Pipe 服务端（cc-helper）
- [ ] 实现 Named Pipe 客户端（cc-helper-hook）
- [ ] 测试双向通信

### Phase 2: UI 适配
- [ ] 实现点击穿透（Region API）
- [ ] 测试多显示器拖拽
- [ ] 适配系统托盘

### Phase 3: 安装与打包
- [ ] 实现 Batch wrapper
- [ ] 适配安装路径
- [ ] 配置 NSIS 打包

### Phase 4: 测试与优化
- [ ] 完整功能测试
- [ ] 性能优化
- [ ] 文档更新

## 技术风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Named Pipe 兼容性问题 | 高 | 充分测试，考虑降级到 TCP |
| 点击穿透不稳定 | 中 | 参考 Tauri 社区实现 |
| Batch 脚本编码问题 | 低 | 使用 UTF-8 编码 |
| Windows 路径长度限制 | 低 | 使用长路径支持 |

## 参考资料

- [Tauri Windows 打包文档](https://tauri.app/v2/guides/building/windows)
- [Windows Named Pipes API](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes)
- [Region API Documentation](https://learn.microsoft.com/en-us/windows/win32/apiwingdi/nf-wingdi-setwindowrgn)
