import { useEffect, useRef } from "react"

const LONG_PRESS_MS = 350
const MOVE_THRESHOLD_PX = 4

export function useHardPress<T extends HTMLElement>(
  onActivate: (() => void) | undefined,
) {
  const ref = useRef<T | null>(null)
  const cbRef = useRef(onActivate)
  cbRef.current = onActivate

  useEffect(() => {
    const el = ref.current
    if (!el || !cbRef.current) return

    let timer: number | undefined
    let startX = 0
    let startY = 0
    let armed = false
    let fired = false

    const fire = () => {
      if (fired) return
      fired = true
      cbRef.current?.()
    }

    const cancel = () => {
      armed = false
      if (timer !== undefined) {
        window.clearTimeout(timer)
        timer = undefined
      }
    }

    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Element | null
      if (target?.closest("button, [role='button'], input, select")) return
      armed = true
      fired = false
      startX = e.clientX
      startY = e.clientY
      timer = window.setTimeout(() => {
        if (armed) fire()
      }, LONG_PRESS_MS)
    }
    const onPointerMove = (e: PointerEvent) => {
      if (!armed) return
      if (
        Math.abs(e.clientX - startX) > MOVE_THRESHOLD_PX ||
        Math.abs(e.clientY - startY) > MOVE_THRESHOLD_PX
      ) {
        cancel()
      }
    }
    // webkitmouseforcedown fires ONCE when the user crosses the OS-level
    // force-click threshold on a pressure-sensitive trackpad. We
    // deliberately do not subscribe to webkitmouseforcechanged — that fires
    // continuously and would trigger on every normal click as pressure
    // ramps through any threshold we set.
    const onForceDown = () => fire()

    el.addEventListener("pointerdown", onPointerDown)
    el.addEventListener("pointermove", onPointerMove)
    el.addEventListener("pointerup", cancel)
    el.addEventListener("pointerleave", cancel)
    el.addEventListener("pointercancel", cancel)
    el.addEventListener(
      "webkitmouseforcedown" as keyof HTMLElementEventMap,
      onForceDown as EventListener,
    )

    return () => {
      cancel()
      el.removeEventListener("pointerdown", onPointerDown)
      el.removeEventListener("pointermove", onPointerMove)
      el.removeEventListener("pointerup", cancel)
      el.removeEventListener("pointerleave", cancel)
      el.removeEventListener("pointercancel", cancel)
      el.removeEventListener(
        "webkitmouseforcedown" as keyof HTMLElementEventMap,
        onForceDown as EventListener,
      )
    }
  }, [])

  return ref
}
