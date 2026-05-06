interface Window {
  __helperResolve?: (allowed: boolean, always?: boolean, answers?: Record<string, string>) => void
  __islandInitStatus?: { status: string; sessionCount: number }
}
