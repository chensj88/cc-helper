<template>
  <div class="island-root" :class="{ expanded }">
    <!-- Collapsed pill -->
    <div
      v-if="!expanded"
      ref="pillEl"
      class="island-pill"
      :class="status"
      @pointerdown="onPillPointerDown"
      @pointermove="onPillPointerMove"
      @pointerup="onPillPointerUp"
    >
      <span class="status-dot" :class="status"></span>
      <span class="pill-text">{{ pillLabel }}</span>
    </div>

    <!-- Expanded panel -->
    <div v-else ref="expandedEl" class="island-expanded" :class="status">

      <!-- Session list mode (click to open) -->
      <template v-if="expandMode === 'sessions'">
        <div
          class="expanded-header"
          @pointerdown="onExpandedHeaderPointerDown"
          @pointermove="onExpandedHeaderPointerMove"
          @pointerup="onExpandedHeaderPointerUp"
        >
          <span class="project-name">Sessions</span>
          <button class="collapse-btn" @click="collapse">&times;</button>
        </div>
        <div class="expanded-body session-list">
          <div v-if="sessions.length === 0" class="empty-hint">No active sessions</div>
          <div
            v-for="s in sessions"
            :key="s.sessionId"
            class="session-row"
          >
            <span class="session-dot" :class="s.status"></span>
            <span class="session-name" :title="s.projectName">{{ s.projectName }}</span>
            <span class="session-status">{{ s.status }}</span>
          </div>
        </div>
      </template>

      <!-- Permission: Command mode -->
      <template v-else-if="mode === 'command'">
        <div
          class="expanded-header"
          @pointerdown="onExpandedHeaderPointerDown"
          @pointermove="onExpandedHeaderPointerMove"
          @pointerup="onExpandedHeaderPointerUp"
        >
          <span class="project-name">{{ pending.projectName }}</span>
          <span class="tool-badge">{{ pending.toolName }}</span>
          <button class="collapse-btn" @click="collapse">&times;</button>
        </div>
        <div class="expanded-body">
          <div class="field" v-if="pending.toolInput.description">
            <span class="label">Description</span>
            <span class="value">{{ pending.toolInput.description }}</span>
          </div>
          <div class="field">
            <span class="label">Command</span>
            <pre class="code">{{ pending.toolInput.command }}</pre>
          </div>
        </div>
        <div class="expanded-footer">
          <button class="btn deny" @click="deny">Deny</button>
          <button v-if="pending.permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
          <button class="btn allow" @click="allow">Allow</button>
        </div>
      </template>

      <!-- Permission: File mode -->
      <template v-else-if="mode === 'file'">
        <div
          class="expanded-header"
          @pointerdown="onExpandedHeaderPointerDown"
          @pointermove="onExpandedHeaderPointerMove"
          @pointerup="onExpandedHeaderPointerUp"
        >
          <span class="project-name">{{ pending.projectName }}</span>
          <span class="tool-badge">{{ pending.toolName }}</span>
          <button class="collapse-btn" @click="collapse">&times;</button>
        </div>
        <div class="expanded-body">
          <div class="field">
            <span class="label">{{ fileOperationLabel }}</span>
            <pre class="code file-path">{{ pending.toolInput.file_path }}</pre>
          </div>
        </div>
        <div class="expanded-footer">
          <button class="btn deny" @click="deny">Deny</button>
          <button v-if="pending.permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
          <button class="btn allow" @click="allow">Allow</button>
        </div>
      </template>

      <!-- Permission: WebFetch mode -->
      <template v-else-if="mode === 'webfetch'">
        <div
          class="expanded-header"
          @pointerdown="onExpandedHeaderPointerDown"
          @pointermove="onExpandedHeaderPointerMove"
          @pointerup="onExpandedHeaderPointerUp"
        >
          <span class="project-name">{{ pending.projectName }}</span>
          <span class="tool-badge">{{ pending.toolName }}</span>
          <button class="collapse-btn" @click="collapse">&times;</button>
        </div>
        <div class="expanded-body">
          <div class="field">
            <span class="label">URL</span>
            <pre class="code">{{ pending.toolInput.url }}</pre>
          </div>
          <div class="field" v-if="pending.toolInput.prompt">
            <span class="label">Prompt</span>
            <span class="value">{{ pending.toolInput.prompt }}</span>
          </div>
        </div>
        <div class="expanded-footer">
          <button class="btn deny" @click="deny">Deny</button>
          <button v-if="pending.permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
          <button class="btn allow" @click="allow">Allow</button>
        </div>
      </template>

      <!-- Permission: Skill mode -->
      <template v-else-if="mode === 'skill'">
        <div
          class="expanded-header"
          @pointerdown="onExpandedHeaderPointerDown"
          @pointermove="onExpandedHeaderPointerMove"
          @pointerup="onExpandedHeaderPointerUp"
        >
          <span class="project-name">{{ pending.projectName }}</span>
          <span class="tool-badge">{{ pending.toolName }}</span>
          <button class="collapse-btn" @click="collapse">&times;</button>
        </div>
        <div class="expanded-body">
          <div class="field">
            <span class="label">Skill</span>
            <span class="value">{{ pending.toolInput.skill }}</span>
          </div>
          <div class="field" v-if="pending.toolInput.args">
            <span class="label">Arguments</span>
            <span class="value">{{ pending.toolInput.args }}</span>
          </div>
        </div>
        <div class="expanded-footer">
          <button class="btn deny" @click="deny">Deny</button>
          <button class="btn allow" @click="allow">Allow</button>
        </div>
      </template>

      <!-- AskUserQuestion mode -->
      <template v-else-if="mode === 'question'">
        <div
          class="expanded-header"
          @pointerdown="onExpandedHeaderPointerDown"
          @pointermove="onExpandedHeaderPointerMove"
          @pointerup="onExpandedHeaderPointerUp"
        >
          <span class="project-name">{{ pending.projectName }}</span>
          <span class="tool-badge">{{ pending.toolName }}</span>
          <button class="collapse-btn" @click="collapse">&times;</button>
        </div>
        <div class="expanded-body question-body">
          <div class="question-text">{{ currentQuestion.question }}</div>
          <div class="options">
            <div
              v-for="(opt, i) in currentQuestion.options"
              :key="i"
              class="option-card"
              :class="{ selected: isSelected(opt.label) }"
              @click="selectOption(opt.label)"
            >
              <div class="option-label">{{ opt.label }}</div>
              <div class="option-desc" v-if="opt.description">{{ opt.description }}</div>
            </div>
            <div
              class="option-card other-option"
              :class="{ selected: otherSelected }"
              @click="selectOther"
            >
              <div class="option-label">Other</div>
              <input
                v-if="otherSelected"
                class="other-input"
                type="text"
                placeholder="Type your answer..."
                v-model="otherText"
                @click.stop
                @input="updateOtherAnswer"
              />
            </div>
          </div>
        </div>
        <div class="expanded-footer">
          <button class="btn deny" @click="deny">Cancel</button>
          <button class="btn allow" @click="submitOrNext" :disabled="!canSubmit">
            {{ isLastQuestion ? 'Submit' : 'Next' }}
          </button>
        </div>
      </template>

      <!-- Fallback mode -->
      <template v-else>
        <div
          class="expanded-header"
          @pointerdown="onExpandedHeaderPointerDown"
          @pointermove="onExpandedHeaderPointerMove"
          @pointerup="onExpandedHeaderPointerUp"
        >
          <span class="project-name">{{ pending.projectName }}</span>
          <span class="tool-badge">{{ pending.toolName }}</span>
          <button class="collapse-btn" @click="collapse">&times;</button>
        </div>
        <div class="expanded-body">
          <div class="field">
            <span class="label">Tool</span>
            <span class="value">{{ pending.toolName }}</span>
          </div>
          <div class="field">
            <span class="label">Input</span>
            <pre class="code">{{ JSON.stringify(pending.toolInput, null, 2).slice(0, 500) }}</pre>
          </div>
        </div>
        <div class="expanded-footer">
          <button class="btn deny" @click="deny">Deny</button>
          <button v-if="pending.permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
          <button class="btn allow" @click="allow">Allow</button>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow, LogicalPosition } from '@tauri-apps/api/window'

// --- Pill drag + click via pointer capture ---
let dragStartX = 0
let dragStartY = 0
let didDrag = false

async function onPillPointerDown(e: PointerEvent) {
  if (expanded.value) return
  const el = e.currentTarget as HTMLElement
  el.setPointerCapture(e.pointerId)
  dragStartX = e.screenX
  dragStartY = e.screenY
  didDrag = false
  try { await invoke('island_set_interactive', { interactive: true }) } catch {}
}

async function onPillPointerMove(e: PointerEvent) {
  if (expanded.value) return
  const dx = e.screenX - dragStartX
  const dy = e.screenY - dragStartY
  if (!didDrag && (Math.abs(dx) < 3 && Math.abs(dy) < 3)) return
  didDrag = true
  dragStartX = e.screenX
  dragStartY = e.screenY
  try {
    const appWindow = getCurrentWindow()
    const pos = await appWindow.outerPosition()
    const scale = await appWindow.scaleFactor()
    await appWindow.setPosition(new LogicalPosition(pos.x / scale + dx, pos.y / scale + dy))
  } catch {}
}

async function onPillPointerUp(e: PointerEvent) {
  const el = e.currentTarget as HTMLElement
  try { el.releasePointerCapture(e.pointerId) } catch {}
  if (didDrag) {
    try {
      const appWindow = getCurrentWindow()
      const pos = await appWindow.outerPosition()
      const scale = await appWindow.scaleFactor()
      await invoke('island_save_position', { x: pos.x / scale, y: pos.y / scale })
    } catch {}
    try { await invoke('island_set_interactive', { interactive: false }) } catch {}
    await updateInputRegion()
  } else {
    await expandForSessions()
  }
}

async function onExpandedHeaderPointerDown(e: PointerEvent) {
  const target = e.target as HTMLElement | null
  if (target?.closest('button, input, textarea, select, a')) return
  const el = e.currentTarget as HTMLElement
  el.setPointerCapture(e.pointerId)
  dragStartX = e.screenX
  dragStartY = e.screenY
  didDrag = false
  try { await invoke('island_set_interactive', { interactive: true }) } catch {}
}

async function onExpandedHeaderPointerMove(e: PointerEvent) {
  const el = e.currentTarget as HTMLElement
  if (!el.hasPointerCapture(e.pointerId)) return
  const dx = e.screenX - dragStartX
  const dy = e.screenY - dragStartY
  if (!didDrag && (Math.abs(dx) < 3 && Math.abs(dy) < 3)) return
  didDrag = true
  dragStartX = e.screenX
  dragStartY = e.screenY
  try {
    const appWindow = getCurrentWindow()
    const pos = await appWindow.outerPosition()
    const scale = await appWindow.scaleFactor()
    await appWindow.setPosition(new LogicalPosition(pos.x / scale + dx, pos.y / scale + dy))
  } catch {}
}

async function onExpandedHeaderPointerUp(e: PointerEvent) {
  const el = e.currentTarget as HTMLElement
  try { el.releasePointerCapture(e.pointerId) } catch {}
  if (!didDrag) return
  try {
    const appWindow = getCurrentWindow()
    const pos = await appWindow.outerPosition()
    const scale = await appWindow.scaleFactor()
    await invoke('island_save_position', { x: pos.x / scale, y: pos.y / scale })
  } catch {}
  try { await invoke('island_set_interactive', { interactive: false }) } catch {}
  await updateInputRegion()
}

// --- State ---
const status = ref('idle')
const sessionCount = ref(0)
const expanded = ref(false)
const pillEl = ref<HTMLElement | null>(null)
const expandedEl = ref<HTMLElement | null>(null)

// expandMode: 'sessions' = user clicked pill, 'permission'|'question' = auto from hook
type ExpandMode = 'sessions' | 'permission'
const expandMode = ref<ExpandMode>('sessions')

interface SessionInfo {
  sessionId: string
  projectName: string
  status: string
}

const sessions = ref<SessionInfo[]>([])

const MIN_SESSIONS_WIDTH = 320
const MAX_SESSIONS_WIDTH = 560
const BASE_NAME_CHARS = 24
const PX_PER_CHAR = 7

interface PendingReq {
  key: string
  toolName: string
  toolInput: Record<string, any>
  projectName: string
  permissionSuggestions?: Array<Record<string, any>>
}

const pending = reactive<PendingReq>({
  key: '',
  toolName: '',
  toolInput: {},
  projectName: '',
})

// --- Question state ---
const questions = computed<Array<{ question: string; options: Array<{ label: string; description?: string }>; multiSelect?: boolean }>>(
  () => pending.toolInput?.questions || []
)
const currentQ = ref(0)
const currentQuestion = computed(() => questions.value[currentQ.value] || { question: '', options: [] })
const isLastQuestion = computed(() => currentQ.value >= questions.value.length - 1)
const answers = reactive<Record<string, string>>({})
const otherSelected = ref(false)
const otherText = ref('')

// --- Computed ---
const pillLabel = computed(() => {
  switch (status.value) {
    case 'working': return sessionCount.value > 0 ? `${sessionCount.value} working` : 'working'
    case 'permission': return 'permission'
    case 'question': return 'question'
    case 'failed': return 'error'
    default: return sessionCount.value > 0 ? `${sessionCount.value} sessions` : 'Helper'
  }
})

const mode = computed(() => {
  switch (pending.toolName) {
    case 'ask_user_question': case 'AskUserQuestion': return 'question'
    case 'Bash': case 'PowerShell': return 'command'
    case 'Edit': case 'Write': case 'Read': case 'Glob': case 'Grep': return 'file'
    case 'WebFetch': return 'webfetch'
    case 'skill': case 'Skill': return 'skill'
    default: return 'fallback'
  }
})

const fileOperationLabel = computed(() => {
  switch (pending.toolName) {
    case 'Edit': return 'Edit File'
    case 'Write': return 'Write File'
    case 'Read': return 'Read File'
    case 'Glob': return 'Search Files'
    case 'Grep': return 'Search Content'
    default: return 'File Operation'
  }
})

// --- Question helpers ---
function isSelected(label: string): boolean {
  const q = currentQuestion.value.question
  return answers[q] === label && !otherSelected.value
}

function selectOption(label: string) {
  const q = currentQuestion.value.question
  answers[q] = label
  otherSelected.value = false
}

function selectOther() {
  otherSelected.value = true
  if (!otherText.value) otherText.value = ''
  const q = currentQuestion.value.question
  answers[q] = otherText.value
}

function updateOtherAnswer() {
  const q = currentQuestion.value.question
  answers[q] = otherText.value
}

const canSubmit = computed(() => {
  const q = currentQuestion.value.question
  return !!answers[q] || !!otherText.value
})

function submitOrNext() {
  if (isLastQuestion.value) {
    doAllow(false, { ...answers })
  } else {
    currentQ.value++
  }
}

// --- Actions ---
async function doAllow(always: boolean, answers?: Record<string, string>) {
  try {
    await invoke('resolve_permission', {
      key: pending.key,
      allowed: true,
      always: always || false,
      answers: answers || null,
    })
  } catch (e) {
    console.error('[island] resolve failed:', e)
  }
  collapse()
}

async function allow() {
  await doAllow(false)
}

async function allowAlways() {
  await doAllow(true)
}

async function deny() {
  try {
    await invoke('resolve_permission', {
      key: pending.key,
      allowed: false,
      answers: null,
    })
  } catch (e) {
    console.error('[island] resolve failed:', e)
  }
  collapse()
}

async function updateInputRegion() {
  await nextTick()
  const el = expanded.value ? expandedEl.value : pillEl.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const pad = 2
  try {
    await invoke('island_set_input_region', {
      x: Math.max(0, Math.floor(rect.x) - pad),
      y: Math.max(0, Math.floor(rect.y) - pad),
      width: Math.ceil(rect.width) + pad * 2,
      height: Math.ceil(rect.height) + pad * 2,
    })
  } catch (e) {
    console.error('[island] set input region failed:', e)
  }
}

async function collapse() {
  expanded.value = false
  try {
    await invoke('island_resize', { width: 160, height: 36 })
  } catch (e) {
    console.error('[island] resize failed:', e)
  }
  invoke('island_set_interactive', { interactive: false }).catch(() => {})
  updateInputRegion()
}

async function expandForSessions() {
  expandMode.value = 'sessions'
  expanded.value = true
  const count = Math.max(sessions.value.length, 1)
  const h = Math.min(60 + count * 36, 300)
  const longestName = sessions.value.reduce((max, s) => Math.max(max, s.projectName.length), 0)
  const estimatedWidth = MIN_SESSIONS_WIDTH + Math.max(0, longestName - BASE_NAME_CHARS) * PX_PER_CHAR
  const width = Math.max(MIN_SESSIONS_WIDTH, Math.min(MAX_SESSIONS_WIDTH, estimatedWidth))
  try {
    await invoke('island_resize', { width, height: h })
    await invoke('island_ensure_visible', { width, height: h })
  } catch (e) {
    console.error('[island] resize failed:', e)
  }
  await updateInputRegion()
  try { await invoke('island_set_interactive', { interactive: false }) } catch {}
}

async function expandForRequest() {
  expandMode.value = 'permission'
  expanded.value = true
  // Reset question state
  currentQ.value = 0
  Object.keys(answers).forEach(k => delete answers[k])
  otherSelected.value = false
  otherText.value = ''

  try {
    await invoke('island_resize', { width: 380, height: 300 })
    await invoke('island_ensure_visible', { width: 380, height: 300 })
  } catch (e) {
    console.error('[island] resize failed:', e)
  }
  await updateInputRegion()
  try { await invoke('island_set_interactive', { interactive: false }) } catch {}
}

// --- Event listeners ---
let unlisteners: UnlistenFn[] = []

onMounted(async () => {
  // Apply initial status fetched by island-main.ts
  const init = (window as any).__islandInitStatus as { status: string; sessionCount: number; sessions: SessionInfo[] } | undefined
  if (init) {
    status.value = init.status
    sessionCount.value = init.sessionCount
    sessions.value = init.sessions || []
  }

  const u1 = await listen<{ status: string; sessionCount: number; sessions: SessionInfo[] }>('island:update-status', (event) => {
    status.value = event.payload.status
    sessionCount.value = event.payload.sessionCount
    sessions.value = event.payload.sessions || []
  })

  const u2 = await listen<PendingReq>('island:show-request', async (event) => {
    const p = event.payload
    pending.key = p.key
    pending.toolName = p.toolName
    pending.toolInput = p.toolInput
    pending.projectName = p.projectName
    pending.permissionSuggestions = p.permissionSuggestions
    status.value = p.toolName === 'ask_user_question' || p.toolName === 'AskUserQuestion' ? 'question' : 'permission'
    await expandForRequest()
  })

  const u3 = await listen('island:resolve', () => {
    collapse()
  })

  unlisteners = [u1, u2, u3]

  await updateInputRegion()

  window.addEventListener('resize', onViewportResize)
})

function onViewportResize() {
  updateInputRegion()
}

watch(pillLabel, async () => {
  await updateInputRegion()
})

onUnmounted(() => {
  window.removeEventListener('resize', onViewportResize)
  unlisteners.forEach(fn => fn())
})
</script>

<style scoped>
.island-root {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  overflow: hidden;
}

/* === Collapsed Pill === */
.island-pill {
  background: rgba(28, 28, 30, 0.85);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-radius: 18px;
  padding: 0 16px;
  height: 36px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: grab;
  user-select: none;
  transition: background 0.3s;
}

.island-pill:active {
  cursor: grabbing;
}

.island-pill.permission {
  box-shadow: 0 0 16px rgba(234, 179, 8, 0.2);
}

.island-pill.question {
  box-shadow: 0 0 16px rgba(59, 130, 246, 0.2);
}

.pill-text {
  color: #e5e7eb;
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
}

/* === Status Dot === */
.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.idle { background: #6b7280; }
.status-dot.working {
  background: #22c55e;
  box-shadow: 0 0 8px rgba(34, 197, 94, 0.5);
  animation: pulse 1.5s ease-in-out infinite;
}
.status-dot.permission {
  background: #eab308;
  box-shadow: 0 0 8px rgba(234, 179, 8, 0.5);
  animation: pulse 1s ease-in-out infinite;
}
.status-dot.question {
  background: #3b82f6;
  box-shadow: 0 0 8px rgba(59, 130, 246, 0.5);
  animation: pulse 1s ease-in-out infinite;
}
.status-dot.failed { background: #ef4444; }

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* === Expanded Panel === */
.island-expanded {
  background: rgba(28, 28, 30, 0.92);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border-radius: 16px;
  width: 100%;
  max-height: 300px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: expandIn 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

@keyframes expandIn {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(-4px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.expanded-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px 6px;
}

.project-name {
  font-weight: 600;
  color: #e5e7eb;
  font-size: 13px;
}

.tool-badge {
  background: rgba(255, 255, 255, 0.08);
  color: #9ca3af;
  border-radius: 4px;
  padding: 1px 6px;
  font-size: 10px;
  font-weight: 500;
}

.island-expanded.permission .tool-badge {
  background: rgba(234, 179, 8, 0.15);
  color: #eab308;
}

.island-expanded.question .tool-badge {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}

.collapse-btn {
  margin-left: auto;
  background: none;
  border: none;
  color: #6b7280;
  font-size: 16px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}

.collapse-btn:hover {
  color: #e5e7eb;
}

/* === Session List === */
.session-list {
  padding: 6px 10px 10px !important;
}

.session-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  transition: background 0.15s;
}

.session-row:hover {
  background: rgba(255, 255, 255, 0.04);
}

.session-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.session-dot.idle { background: #6b7280; }
.session-dot.working { background: #22c55e; }
.session-dot.waitingpermission { background: #eab308; }
.session-dot.failed { background: #ef4444; }

.session-name {
  color: #e5e7eb;
  font-size: 13px;
  font-weight: 500;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-status {
  color: #6b7280;
  font-size: 11px;
  flex-shrink: 0;
  min-width: 46px;
  text-align: right;
}

.empty-hint {
  color: #6b7280;
  font-size: 13px;
  text-align: center;
  padding: 12px 0;
}

/* === Body === */
.expanded-body {
  flex: 1;
  overflow-y: auto;
  padding: 6px 14px;
}

.question-body {
  padding: 10px 14px;
}

.field {
  margin-bottom: 8px;
}

.field .label {
  display: block;
  font-size: 11px;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 4px;
}

.field .value {
  color: #e5e7eb;
  font-size: 13px;
}

.code {
  background: #2c2c2e;
  border-radius: 8px;
  padding: 8px 10px;
  margin: 0;
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 12px;
  color: #e5e7eb;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 120px;
  overflow-y: auto;
}

.file-path {
  word-break: break-all;
}

/* === Question Options === */
.question-text {
  font-size: 14px;
  font-weight: 500;
  color: #e5e7eb;
  margin-bottom: 10px;
}

.options {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.option-card {
  background: #2c2c2e;
  border-radius: 8px;
  padding: 8px 10px;
  border: 2px solid transparent;
  cursor: pointer;
  transition: border-color 0.15s;
}

.option-card:hover {
  border-color: rgba(255, 255, 255, 0.1);
}

.option-card.selected {
  border-color: #3b82f6;
}

.option-card .option-label {
  font-weight: 500;
  color: #e5e7eb;
  font-size: 13px;
}

.option-card.selected .option-label {
  color: #60a5fa;
}

.option-card .option-desc {
  font-size: 11px;
  color: #9ca3af;
  margin-top: 2px;
}

.other-input {
  width: 100%;
  margin-top: 4px;
  background: #1c1c1e;
  border: 1px solid #555;
  border-radius: 4px;
  padding: 4px 6px;
  color: #fff;
  font-size: 12px;
  outline: none;
}

.other-input:focus {
  border-color: #3b82f6;
}

/* === Footer === */
.expanded-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 8px 14px 12px;
}

.btn {
  border: none;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.15s;
}

.btn:hover { opacity: 0.85; }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }

.btn.deny {
  background: #3a3a3c;
  color: #e5e7eb;
}

.btn.allow {
  background: #22c55e;
  color: #fff;
}

.btn.always-allow {
  background: #3b82f6;
  color: #fff;
}

.btn.always-allow:hover {
  background: #2563eb;
}

.island-expanded.question .btn.allow {
  background: #3b82f6;
}
</style>
