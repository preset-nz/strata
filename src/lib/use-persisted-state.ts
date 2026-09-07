import { useCallback, useEffect, useState } from "react"

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

  const setValue = useCallback(
    (next: T | ((prev: T) => T)) => {
      setValueRaw((prev) => {
        return typeof next === "function"
          ? (next as (prev: T) => T)(prev)
          : next
      })
    },
    [],
  )

  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(value))
    } catch {
      // Quota or serialisation failure — UI state is lossy by nature.
    }
  }, [key, value])

  return [value, setValue]
}
