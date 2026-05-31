import { createApp } from 'vue'
import DynamicIsland from './components/DynamicIsland.vue'
import { invoke } from '@tauri-apps/api/core'

// Fetch initial status on load (events before mount are lost)
invoke<{ status: string; sessionCount: number; sessions: Array<{ sessionId: string; projectName: string; status: string }> }>('get_island_status')
  .then((payload) => {
    window.__islandInitStatus = payload
  })
  .catch(() => {})
  .finally(() => {
    createApp(DynamicIsland).mount('#app')
  })