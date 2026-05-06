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

const suggestionsStr = params.get('suggestions') || '[]'
let permissionSuggestions: Array<Record<string, any>> = []
try { permissionSuggestions = JSON.parse(suggestionsStr) } catch (e) {}
const app = createApp(PermissionDialog, { toolName, toolInput, projectName: project, permissionSuggestions })
app.mount('#app')

window.__helperResolve = async (allowed: boolean, always?: boolean, answers?: Record<string, string>) => {
  try {
    await invoke('resolve_permission', {
      key,
      allowed,
      always: always || false,
      answers: answers || null,
    })
  } catch (e) {
    console.error('[dialog] invoke failed:', e)
  }
}
