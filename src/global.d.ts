interface Window {
  __helperResolve?: (allowed: boolean, answers?: Record<string, string>) => void
  __islandInitStatus?: { status: string; sessionCount: number }
}
