// Shortcuts bound on `window` fire regardless of focus. Anything that types a
// character — or a select, which eats arrow keys — must win over them.
export function isTypingTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  if (!el || typeof el.tagName !== "string") return false
  if (el.isContentEditable) return true
  const tag = el.tagName.toLowerCase()
  return tag === "input" || tag === "textarea" || tag === "select"
}
