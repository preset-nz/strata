import { invoke } from "@tauri-apps/api/core"
import { useEffect, useState } from "react"

import { cn } from "@/lib/utils"
import { VGA16_LABEL, type Vga16Bucket } from "@/lib/vga16"

const KS = [4, 8, 16] as const
type K = (typeof KS)[number]

type Swatch = {
  hex: string
  lab: [number, number, number]
  lch: [number, number, number]
  weight: number
  bucket: Vga16Bucket
}

// Results by image and k, so reopening the group or switching back is free.
const cache = new Map<string, Swatch[]>()

function lchText([l, c, h]: [number, number, number]): string {
  return `L ${l.toFixed(0)} C ${c.toFixed(0)} h ${h.toFixed(0)}°`
}

/**
 * The palette tool: k swatches for one image, computed on request and not
 * stored. Click a hex or an LCh value to copy it.
 */
export function PaletteExtract({ imageId }: { imageId: string }) {
  // Nothing runs until a k is picked: the panel shows every group open.
  const [k, setK] = useState<K | null>(null)
  const key = k === null ? null : `${imageId}:${k}`
  const [result, setResult] = useState<{
    key: string | null
    swatches: Swatch[] | null
    error: string | null
  }>({ key: null, swatches: null, error: null })
  const [copied, setCopied] = useState<string | null>(null)

  useEffect(() => {
    if (key === null || k === null) return
    const cached = cache.get(key)
    if (cached) {
      // cached result for a new key
      setResult({ key, swatches: cached, error: null })
      return
    }
    let cancelled = false
    invoke<Swatch[]>("extract_palette", { id: imageId, k })
      .then((swatches) => {
        cache.set(key, swatches)
        if (!cancelled) setResult({ key, swatches, error: null })
      })
      .catch((e) => {
        if (!cancelled) setResult({ key, swatches: null, error: String(e) })
      })
    return () => {
      cancelled = true
    }
  }, [imageId, k, key])

  const copy = (text: string) => {
    void navigator.clipboard.writeText(text).then(() => setCopied(text))
  }

  const current = result.key === key ? result : { swatches: null, error: null }

  return (
    <div className="flex flex-col gap-2">
      <fieldset className="m-0 flex min-w-0 gap-1 border-0 p-0">
        <legend className="sr-only">Number of colours</legend>
        {KS.map((n) => (
          <button
            key={n}
            type="button"
            aria-pressed={k === n}
            onClick={() => setK(n)}
            className={cn(
              "rounded-sm border px-2 py-0.5 text-xs tabular-nums",
              k === n
                ? "border-primary bg-primary text-primary-foreground"
                : "border-border hover:bg-muted"
            )}
          >
            {n}
          </button>
        ))}
      </fieldset>
      {k === null ? (
        <p className="text-muted-foreground text-xs">
          Pick how many colours to extract.
        </p>
      ) : current.error ? (
        <p className="select-text text-destructive text-xs">{current.error}</p>
      ) : !current.swatches ? (
        <p className="text-muted-foreground text-xs">Extracting…</p>
      ) : (
        <>
          <div
            className="flex h-4 w-full overflow-hidden border border-border"
            aria-hidden
          >
            {current.swatches.map((s) => (
              <span
                key={`${s.hex}-${s.weight}`}
                style={{ backgroundColor: s.hex, flexGrow: s.weight }}
              />
            ))}
          </div>
          <ul className="flex flex-col gap-0.5">
            {current.swatches.map((s) => (
              <li
                key={`${s.hex}-${s.weight}`}
                className="flex items-center gap-2 text-xs"
              >
                <span
                  aria-hidden
                  className="inline-block size-4 shrink-0 border border-border"
                  style={{ backgroundColor: s.hex }}
                />
                <button
                  type="button"
                  onClick={() => copy(s.hex)}
                  title="Copy hex"
                  className="font-mono hover:underline"
                >
                  {copied === s.hex ? "copied" : s.hex}
                </button>
                <button
                  type="button"
                  onClick={() => copy(lchText(s.lch))}
                  title="Copy LCh"
                  className="truncate text-[10px] text-muted-foreground hover:underline"
                >
                  {lchText(s.lch)}
                </button>
                <span className="ml-auto shrink-0 text-[10px] text-muted-foreground tabular-nums">
                  {(s.weight * 100).toFixed(0)}% ·{" "}
                  {VGA16_LABEL[s.bucket] ?? s.bucket}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  )
}
