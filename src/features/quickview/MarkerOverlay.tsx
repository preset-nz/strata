import { invoke } from "@tauri-apps/api/core"
import { useEffect, useState } from "react"

import { VGA16_LABEL, type Vga16Bucket } from "@/lib/vga16"

type Marker = {
  hex: string
  lch: [number, number, number]
  weight: number
  bucket: Vga16Bucket
  x: number
  y: number
}

function lchText([l, c, h]: [number, number, number]): string {
  return `L ${l.toFixed(0)} C ${c.toFixed(0)} h ${h.toFixed(0)}°`
}

/**
 * Dots over the image where each prominent colour's largest region is.
 * Click copies the hex; Option-click copies the LCh. Positioned in
 * fractions of the image box, so it sits over the image at any size.
 */
export function MarkerOverlay({
  imageId,
  count,
}: {
  imageId: string
  count: number
}) {
  const key = `${imageId}:${count}`
  const [result, setResult] = useState<{
    key: string
    markers: Marker[]
  } | null>(null)
  const [copied, setCopied] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    invoke<Marker[]>("palette_markers", { id: imageId, n: count })
      .then((markers) => {
        if (!cancelled) setResult({ key, markers })
      })
      .catch((e) => console.error("palette markers failed:", e))
    return () => {
      cancelled = true
    }
  }, [imageId, count, key])

  if (result?.key !== key) return null

  return (
    <div className="pointer-events-none absolute inset-0">
      {result.markers.map((m, i) => {
        const label = `${VGA16_LABEL[m.bucket] ?? m.bucket} · ${m.hex} · ${lchText(m.lch)} · ${(m.weight * 100).toFixed(0)}%`
        return (
          <button
            key={i}
            type="button"
            aria-label={`Copy ${label}`}
            title={
              copied === m.hex
                ? "Copied"
                : `${label}\nClick: copy hex · Option-click: copy LCh`
            }
            onClick={(e) => {
              e.stopPropagation()
              const text = e.altKey ? lchText(m.lch) : m.hex
              void navigator.clipboard
                .writeText(text)
                .then(() => setCopied(m.hex))
            }}
            className="pointer-events-auto absolute size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7),0_1px_3px_rgba(0,0,0,0.5)] transition-transform hover:scale-125 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            style={{
              left: `${m.x * 100}%`,
              top: `${m.y * 100}%`,
              backgroundColor: m.hex,
            }}
          />
        )
      })}
    </div>
  )
}
