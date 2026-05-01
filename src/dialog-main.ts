import { createApp } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import PermissionDialog from './components/PermissionDialog.vue'

const params = new URLSearchParams(window.location.search)
const toolName = params.get('tool_name') || 'unknown'
const project = params.get('project') || ''
const key = params.get('key') || ''
const inputStr = params.get('input') || '{}'
let toolInput = {}
try { toolInput = JSON.parse(inputStr) } catch (e) {}

const app = createApp(PermissionDialog, { toolName, toolInput, projectName: project })
app.mount('#app')

window.__helperResolve = async (allowed: boolean, answers?: Record<string, string>) => {
  try {
    await invoke('resolve_permission', {
      key,
      allowed,
      answers: answers || null,
    })
  } catch (e) {
    console.error('[dialog] invoke failed:', e)
  }
}
