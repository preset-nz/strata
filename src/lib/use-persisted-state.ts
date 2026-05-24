import { useCallback, useEffect, useRef, useState } from "react"

// Right home for UI ephemera (rail collapse, sort direction per view, etc.).
// When epic 12 lands a settings store, swap the body of this hook; call sites
// stay the same.

export function usePersistedState<T>(
  key: string,
  initial: T,
): [T, (next: T | ((prev: T) => T)) => void] {
  const [value, setValueRaw] = useState<T>(() => {
    try {
      const raw = localStorage.getItem(key)
      if (raw === null) return initial
      return JSON.parse(raw) as T
    } catch {
      return initial
    }
  })

  const latest = useRef(value)
  latest.current = value

  const setValue = useCallback(
    (next: T | ((prev: T) => T)) => {
      setValueRaw((prev) => {
        const resolved =
          typeof next === "function"
            ? (next as (prev: T) => T)(prev)
            : next
        latest.current = resolved
        return resolved
      })
    },
    [],
  )

  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(latest.current))
    } catch {
      // Quota or serialisation failure — UI state is lossy by nature.
    }
  }, [key, value])

  return [value, setValue]
}
