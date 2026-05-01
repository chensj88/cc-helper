import { createApp } from 'vue'
import DynamicIsland from './components/DynamicIsland.vue'
import { invoke } from '@tauri-apps/api/core'

const app = createApp(DynamicIsland)
app.mount('#app')

// Fetch initial status on load (events before mount are lost)
invoke<{ status: string; sessionCount: number }>('get_island_status')
  .then((payload) => {
    window.__islandInitStatus = payload
  })
  .catch(() => {})
