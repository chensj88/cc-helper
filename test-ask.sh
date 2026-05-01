#!/usr/bin/env bash
# Test script: send mock AskUserQuestion / PermissionRequest events to the helper daemon
# Usage: ./test-ask.sh [question|permission|command|file]

SOCKET="/tmp/claude-code-helper.sock"

send_event() {
  local event="$1"
  local payload="$2"
  # Compact JSON to single line — IPC reads one line per request
  local compacted
  compacted=$(echo "$payload" | tr -d '\n' | sed 's/  */ /g')
  local request="{\"version\":1,\"event\":\"${event}\",\"session_id\":\"test-$$\",\"cwd\":\"/tmp/test-project\",\"payload\":${compacted}}"
  echo "$request" | socat - UNIX-CONNECT:"$SOCKET" 2>/dev/null
}

case "${1:-question}" in

  question)
    send_event "permission_request" '{
      "tool_name": "AskUserQuestion",
      "tool_input": {
        "questions": [
          {
            "question": "你想使用哪个前端框架？",
            "options": [
              {"label": "React", "description": "Meta 开发的声明式 UI 库"},
              {"label": "Vue", "description": "渐进式 JavaScript 框架"},
              {"label": "Svelte", "description": "编译时框架，无虚拟 DOM"},
              {"label": "Angular", "description": "Google 的企业级框架"}
            ]
          }
        ]
      },
      "tool_use_id": "test_ask_001"
    }'
    ;;

  multi-question)
    send_event "permission_request" '{
      "tool_name": "AskUserQuestion",
      "tool_input": {
        "questions": [
          {
            "question": "选择数据库类型？",
            "options": [
              {"label": "PostgreSQL", "description": "功能强大的开源关系数据库"},
              {"label": "MySQL", "description": "流行的开源关系数据库"},
              {"label": "MongoDB", "description": "NoSQL 文档数据库"}
            ]
          },
          {
            "question": "选择部署方式？",
            "options": [
              {"label": "Docker", "description": "容器化部署"},
              {"label": "Kubernetes", "description": "容器编排平台"},
              {"label": "裸机部署", "description": "直接部署到服务器"}
            ]
          }
        ]
      },
      "tool_use_id": "test_ask_002"
    }'
    ;;

  permission)
    send_event "permission_request" '{
      "tool_name": "Bash",
      "tool_input": {
        "command": "npm run build && npm run deploy -- --prod",
        "description": "构建并部署到生产环境"
      },
      "tool_use_id": "test_perm_001"
    }'
    ;;

  command)
    send_event "permission_request" '{
      "tool_name": "Bash",
      "tool_input": {
        "command": "rm -rf node_modules && npm install",
        "description": "清理并重新安装依赖"
      },
      "tool_use_id": "test_perm_002"
    }'
    ;;

  file)
    send_event "permission_request" '{
      "tool_name": "Write",
      "tool_input": {
        "file_path": "/tmp/test-project/src/config.ts"
      },
      "tool_use_id": "test_perm_003"
    }'
    ;;

  webfetch)
    send_event "permission_request" '{
      "tool_name": "WebFetch",
      "tool_input": {
        "url": "https://api.example.com/v1/users",
        "prompt": "Extract all user names and emails from the response"
      },
      "tool_use_id": "test_perm_004"
    }'
    ;;

  skill)
    send_event "permission_request" '{
      "tool_name": "Skill",
      "tool_input": {
        "skill": "create-readme",
        "args": "--lang zh-CN"
      },
      "tool_use_id": "test_perm_005"
    }'
    ;;

  fruit)
    send_event "permission_request" '{
      "tool_name": "AskUserQuestion",
      "tool_input": {
        "questions": [
          {
            "question": "你想吃哪种水果？",
            "options": [
              {"label": "香蕉", "description": "黄色的弯弯的水果"},
              {"label": "苹果", "description": "红色的圆圆的水果"},
              {"label": "梨", "description": "黄绿色的水水的果"}
            ]
          }
        ]
      },
      "tool_use_id": "fruit_001"
    }'
    ;;

  session)
    send_event "session_start" '{}'
    ;;

  stop)
    send_event "stop" '{}'
    ;;

  *)
    echo "Usage: $0 [question|multi-question|fruit|permission|command|file|webfetch|skill|session|stop]"
    exit 1
    ;;

esac
