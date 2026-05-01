<template>
  <div class="session-panel">
    <div class="panel-header">Claude Code Sessions</div>
    <div v-if="sessions.length === 0" class="empty">No active sessions</div>
    <div v-for="session in sessions" :key="session.sessionId" class="session-item">
      <span class="status-dot" :class="statusClass(session.status)"></span>
      <span class="project">{{ session.projectName }}</span>
      <span class="status-text">{{ statusText(session.status) }}</span>
      <span class="sid">{{ session.sessionId.slice(0, 6) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ sessions: Array<any> }>()
function statusClass(status: string) {
  return { working: status === 'Working', waiting: status === 'WaitingPermission', failed: status === 'Failed', idle: status === 'Idle' }
}
function statusText(status: string) {
  const map: Record<string, string> = { Working: 'working', WaitingPermission: 'perm', Failed: 'failed', Idle: 'idle' }
  return map[status] || status
}
</script>

<style scoped>
.session-panel { padding: 12px; min-width: 280px; }
.panel-header { font-weight: 600; margin-bottom: 8px; font-size: 14px; }
.empty { color: #888; font-size: 13px; }
.session-item { display: flex; align-items: center; gap: 8px; padding: 4px 0; font-size: 13px; }
.status-dot { width: 8px; height: 8px; border-radius: 50%; }
.status-dot.working { background: #22c55e; }
.status-dot.waiting { background: #eab308; animation: pulse 1s infinite; }
.status-dot.failed { background: #ef4444; }
.status-dot.idle { background: #888; }
.project { font-weight: 500; }
.status-text { color: #888; }
.sid { color: #aaa; font-family: monospace; font-size: 11px; margin-left: auto; }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
</style>
