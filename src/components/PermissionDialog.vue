<template>
  <div class="permission-dialog">
    <!-- AskUserQuestion mode -->
    <template v-if="mode === 'question'">
      <div class="dialog-header">
        <span class="title">Question</span>
        <span class="badge" v-if="questions.length > 1">{{ currentQ + 1 }} / {{ questions.length }}</span>
      </div>
      <div class="dialog-body">
        <div class="question-text">{{ currentQuestion.question }}</div>
        <div v-if="currentQuestion.header" class="question-header">{{ currentQuestion.header }}</div>
        <div class="options">
          <div
            v-for="(opt, i) in currentQuestion.options"
            :key="i"
            class="option-card"
            :class="{ selected: isSelected(currentQuestion.question, opt.label) }"
            @click="selectOption(currentQuestion.question, opt.label)"
          >
            <div class="option-label">{{ opt.label }}</div>
            <div class="option-desc" v-if="opt.description">{{ opt.description }}</div>
          </div>
          <div
            class="option-card other-option"
            :class="{ selected: isOtherSelected(currentQuestion.question) }"
            @click="selectOther(currentQuestion.question)"
          >
            <div class="option-label">Other</div>
            <input
              v-if="isOtherSelected(currentQuestion.question)"
              class="other-input"
              type="text"
              placeholder="Type your answer..."
              v-model="otherTexts[currentQuestion.question]"
              @click.stop
              @input="updateOtherAnswer(currentQuestion.question)"
            />
          </div>
        </div>
      </div>
      <div class="dialog-footer">
        <button class="btn deny" @click="deny">Cancel</button>
        <button class="btn allow" @click="submitOrNext" :disabled="!canSubmit">
          {{ isLastQuestion ? 'Submit' : 'Next' }}
        </button>
      </div>
    </template>

    <!-- Command mode (Bash, PowerShell) -->
    <template v-else-if="mode === 'command'">
      <div class="dialog-header">
        <span class="title">Permission Request</span>
        <span class="badge tool">{{ toolName }}</span>
      </div>
      <div class="dialog-body">
        <div class="field" v-if="toolInput.description">
          <span class="label">Description</span>
          <span class="value">{{ toolInput.description }}</span>
        </div>
        <div class="field">
          <span class="label">Command</span>
          <pre class="code">{{ toolInput.command }}</pre>
        </div>
        <div class="field">
          <span class="label">Project</span>
          <span class="value">{{ projectName }}</span>
        </div>
      </div>
      <div class="dialog-footer">
        <button class="btn deny" @click="deny">Deny</button>
        <button v-if="permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
        <button class="btn allow" @click="allow">Allow</button>
      </div>
    </template>

    <!-- File mode (Edit, Write, Read, Glob, Grep) -->
    <template v-else-if="mode === 'file'">
      <div class="dialog-header">
        <span class="title">Permission Request</span>
        <span class="badge tool">{{ fileOperationLabel }}</span>
      </div>
      <div class="dialog-body">
        <div class="field">
          <span class="label">{{ fileOperationLabel }}</span>
          <pre class="code file-path">{{ toolInput.file_path }}</pre>
        </div>
        <div class="field">
          <span class="label">Project</span>
          <span class="value">{{ projectName }}</span>
        </div>
      </div>
      <div class="dialog-footer">
        <button class="btn deny" @click="deny">Deny</button>
        <button v-if="permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
        <button class="btn allow" @click="allow">Allow</button>
      </div>
    </template>

    <!-- WebFetch mode -->
    <template v-else-if="mode === 'webfetch'">
      <div class="dialog-header">
        <span class="title">Permission Request</span>
        <span class="badge tool">WebFetch</span>
      </div>
      <div class="dialog-body">
        <div class="field">
          <span class="label">URL</span>
          <pre class="code">{{ toolInput.url }}</pre>
        </div>
        <div class="field" v-if="toolInput.prompt">
          <span class="label">Prompt</span>
          <span class="value">{{ toolInput.prompt }}</span>
        </div>
      </div>
      <div class="dialog-footer">
        <button class="btn deny" @click="deny">Deny</button>
        <button v-if="permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
        <button class="btn allow" @click="allow">Allow</button>
      </div>
    </template>

    <!-- Skill mode -->
    <template v-else-if="mode === 'skill'">
      <div class="dialog-header">
        <span class="title">Permission Request</span>
        <span class="badge tool">Skill</span>
      </div>
      <div class="dialog-body">
        <div class="field">
          <span class="label">Skill</span>
          <span class="value">{{ toolInput.skill }}</span>
        </div>
        <div class="field" v-if="toolInput.args">
          <span class="label">Arguments</span>
          <span class="value">{{ toolInput.args }}</span>
        </div>
      </div>
      <div class="dialog-footer">
        <button class="btn deny" @click="deny">Deny</button>
        <button class="btn allow" @click="allow">Allow</button>
      </div>
    </template>

    <!-- Fallback mode -->
    <template v-else>
      <div class="dialog-header">
        <span class="title">Permission Request</span>
        <span class="badge tool">{{ toolName }}</span>
      </div>
      <div class="dialog-body">
        <div class="field">
          <span class="label">Tool</span>
          <span class="value">{{ toolName }}</span>
        </div>
        <div class="field">
          <span class="label">Input</span>
          <pre class="code">{{ JSON.stringify(toolInput, null, 2).slice(0, 500) }}</pre>
        </div>
        <div class="field">
          <span class="label">Project</span>
          <span class="value">{{ projectName }}</span>
        </div>
      </div>
      <div class="dialog-footer">
        <button class="btn deny" @click="deny">Deny</button>
        <button v-if="permissionSuggestions?.length" class="btn always-allow" @click="allowAlways">Always Allow</button>
        <button class="btn allow" @click="allow">Allow</button>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

const props = defineProps<{
  toolName: string
  toolInput: Record<string, any>
  projectName: string
  permissionSuggestions?: Array<Record<string, any>>
}>()

interface Question {
  question: string
  header?: string
  options: Array<{ label: string; description?: string; preview?: string }>
  multiSelect?: boolean
}

const mode = computed(() => {
  switch (props.toolName) {
    case 'ask_user_question': case 'AskUserQuestion': return 'question'
    case 'Bash': case 'PowerShell': return 'command'
    case 'Edit': case 'Write': case 'Read': case 'Glob': case 'Grep': return 'file'
    case 'WebFetch': return 'webfetch'
    case 'skill': case 'Skill': return 'skill'
    default: return 'fallback'
  }
})

const fileOperationLabel = computed(() => {
  switch (props.toolName) {
    case 'Edit': return 'Edit File'
    case 'Write': return 'Write File'
    case 'Read': return 'Read File'
    case 'Glob': return 'Search Files'
    case 'Grep': return 'Search Content'
    default: return 'File Operation'
  }
})

// AskUserQuestion state
const questions = computed<Question[]>(() => props.toolInput.questions || [])
const currentQ = ref(0)
const currentQuestion = computed(() => questions.value[currentQ.value] || { question: '', options: [] })
const isLastQuestion = computed(() => currentQ.value >= questions.value.length - 1)

const answers = reactive<Record<string, string>>({})
const otherSelected = reactive<Record<string, boolean>>({})
const otherTexts = reactive<Record<string, string>>({})

function isSelected(question: string, label: string): boolean {
  return answers[question] === label && !otherSelected[question]
}

function isOtherSelected(question: string): boolean {
  return !!otherSelected[question]
}

function selectOption(question: string, label: string) {
  answers[question] = label
  otherSelected[question] = false
}

function selectOther(question: string) {
  otherSelected[question] = true
  if (!otherTexts[question]) otherTexts[question] = ''
  answers[question] = otherTexts[question] || ''
}

function updateOtherAnswer(question: string) {
  answers[question] = otherTexts[question] || ''
}

const canSubmit = computed(() => {
  const q = currentQuestion.value
  if (!q.question) return false
  return !!answers[q.question] || !!otherTexts[q.question]
})

function submitOrNext() {
  if (isLastQuestion.value) {
    // Submit all answers
    if (window.__helperResolve) window.__helperResolve(true, false, { ...answers })
  } else {
    currentQ.value++
  }
}

function allow() {
  if (window.__helperResolve) window.__helperResolve(true, false)
}

function allowAlways() {
  if (window.__helperResolve) window.__helperResolve(true, true)
}

function deny() {
  if (window.__helperResolve) window.__helperResolve(false, false)
}
</script>
