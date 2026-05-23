/* eslint-disable react-refresh/only-export-components --
 * This module ships the built-in field renderers as small co-located
 * components plus a single `registerBuiltinRenderers` entry point. Splitting
 * each renderer into its own file would be over-organisation for code that
 * never hot-reloads in isolation. */
import { useMemo } from "react"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Checkbox } from "@/components/ui/checkbox"
import { registerFieldRenderer } from "./registry"
import type {
  CheckboxFieldDef,
  ColorFieldDef,
  FieldRenderer,
  FieldRendererProps,
  NumberFieldDef,
  SelectFieldDef,
  SliderFieldDef,
  TextFieldDef,
  TextareaFieldDef,
  FileFieldDef,
  VectorFieldDef,
} from "./types"

function ReadOnlyText({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-8 px-0 py-1 text-xs text-foreground tabular-nums break-all">
      {children}
    </div>
  )
}

function Empty() {
  return <span className="text-muted-foreground">—</span>
}

function FieldShell({
  label,
  children,
}: {
  label?: string
  children: React.ReactNode
}) {
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <Label className="text-[11px] font-medium text-muted-foreground tracking-wide">
          {label}
        </Label>
      )}
      {children}
    </div>
  )
}

function formatPrimitive(value: unknown): string {
  if (value == null || value === "") return ""
  if (typeof value === "boolean") return value ? "Yes" : "No"
  if (value instanceof Date) return value.toISOString()
  if (typeof value === "object") return JSON.stringify(value)
  return String(value)
}

const TextRenderer: FieldRenderer<TextFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
}) => {
  const display = formatPrimitive(value)
  return (
    <FieldShell label={field.label ?? field.id}>
      {onChange ? (
        <Input
          aria-label={field.label ?? field.id}
          value={display}
          placeholder={field.placeholder}
          disabled={disabled}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : display ? (
        <ReadOnlyText>{display}</ReadOnlyText>
      ) : (
        <ReadOnlyText>
          <Empty />
        </ReadOnlyText>
      )}
    </FieldShell>
  )
}

const TextareaRenderer: FieldRenderer<TextareaFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
}) => {
  const display = formatPrimitive(value)
  return (
    <FieldShell label={field.label ?? field.id}>
      {onChange ? (
        <textarea
          aria-label={field.label ?? field.id}
          value={display}
          rows={field.rows ?? 3}
          disabled={disabled}
          placeholder={field.placeholder}
          onChange={(e) => onChange(e.target.value)}
          className="w-full rounded-none border border-input bg-transparent px-2.5 py-1 text-xs outline-none focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/50 disabled:opacity-50"
        />
      ) : display ? (
        <ReadOnlyText>
          <span className="whitespace-pre-wrap">{display}</span>
        </ReadOnlyText>
      ) : (
        <ReadOnlyText>
          <Empty />
        </ReadOnlyText>
      )}
    </FieldShell>
  )
}

const NumberRenderer: FieldRenderer<NumberFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
}) => {
  const num =
    typeof value === "number"
      ? value
      : value != null && value !== ""
      ? Number(value)
      : null
  const display =
    num != null && Number.isFinite(num) ? num.toLocaleString() : ""
  return (
    <FieldShell label={field.label ?? field.id}>
      {onChange ? (
        <Input
          type="number"
          aria-label={field.label ?? field.id}
          value={num != null && Number.isFinite(num) ? String(num) : ""}
          min={field.min}
          max={field.max}
          step={field.step ?? 1}
          disabled={disabled}
          onChange={(e) => onChange(Number(e.target.value))}
        />
      ) : display ? (
        <ReadOnlyText>{display}</ReadOnlyText>
      ) : (
        <ReadOnlyText>
          <Empty />
        </ReadOnlyText>
      )}
    </FieldShell>
  )
}

const SliderRenderer: FieldRenderer<SliderFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
}) => {
  const num = typeof value === "number" ? value : Number(value ?? 0)
  return (
    <FieldShell label={field.label ?? field.id}>
      {onChange ? (
        <input
          type="range"
          aria-label={field.label ?? field.id}
          value={Number.isFinite(num) ? num : (field.min ?? 0)}
          min={field.min}
          max={field.max}
          step={field.step ?? 1}
          disabled={disabled}
          onChange={(e) => onChange(Number(e.target.value))}
          className="w-full"
        />
      ) : (
        <ReadOnlyText>
          {Number.isFinite(num) ? num.toLocaleString() : <Empty />}
        </ReadOnlyText>
      )}
    </FieldShell>
  )
}

const SelectRenderer: FieldRenderer<SelectFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
  ctx,
}) => {
  const options = useMemo(
    () => (field.optionsProvider ? field.optionsProvider(ctx) : (field.options ?? [])),
    [field, ctx],
  )
  const current = value == null ? "" : String(value)
  const matched = options.find((o) => o.value === current)
  if (!onChange) {
    return (
      <FieldShell label={field.label ?? field.id}>
        <ReadOnlyText>
          {matched ? matched.label : current ? current : <Empty />}
        </ReadOnlyText>
      </FieldShell>
    )
  }
  return (
    <FieldShell label={field.label ?? field.id}>
      <Select
        value={current}
        onValueChange={(v) => onChange(v)}
        disabled={disabled}
      >
        <SelectTrigger>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </FieldShell>
  )
}

const ColorRenderer: FieldRenderer<ColorFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
}) => {
  const hex = typeof value === "string" && value.length > 0 ? value : null
  return (
    <FieldShell label={field.label ?? field.id}>
      <div className="flex items-center gap-2">
        {hex ? (
          <span
            aria-hidden
            className="inline-block size-4 shrink-0 border border-border"
            style={{ backgroundColor: hex }}
          />
        ) : (
          <span className="inline-block size-4 shrink-0 border border-dashed border-border" />
        )}
        {onChange ? (
          <Input
            type="text"
            aria-label={field.label ?? field.id}
            value={hex ?? ""}
            disabled={disabled}
            onChange={(e) => onChange(e.target.value)}
            className="font-mono"
          />
        ) : (
          <ReadOnlyText>
            <span className="font-mono">{hex ?? <Empty />}</span>
          </ReadOnlyText>
        )}
      </div>
    </FieldShell>
  )
}

const CheckboxRenderer: FieldRenderer<CheckboxFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
}) => {
  const checked = Boolean(value)
  if (!onChange) {
    return (
      <FieldShell label={field.label ?? field.id}>
        <ReadOnlyText>{checked ? "Yes" : "No"}</ReadOnlyText>
      </FieldShell>
    )
  }
  return (
    <div className="flex items-center gap-2">
      <Checkbox
        checked={checked}
        disabled={disabled}
        onCheckedChange={(next) => onChange(Boolean(next))}
      />
      {field.label && (
        <Label className="text-xs text-foreground">{field.label}</Label>
      )}
    </div>
  )
}

const FileRenderer: FieldRenderer<FileFieldDef> = ({
  field,
  value,
}) => {
  const asset = value as { name?: string } | null | undefined
  return (
    <FieldShell label={field.label ?? field.id}>
      <ReadOnlyText>
        {asset?.name ? asset.name : <Empty />}
      </ReadOnlyText>
    </FieldShell>
  )
}

function toNumberArray(value: unknown, arity: number): Array<number | null> {
  if (!Array.isArray(value)) return Array.from({ length: arity }, () => null)
  return Array.from({ length: arity }, (_, i) => {
    const v = value[i]
    if (v == null || v === "") return null
    const n = Number(v)
    return Number.isFinite(n) ? n : null
  })
}

function formatComponent(
  n: number | null,
  integer: boolean,
  precision?: number,
): string {
  if (n == null) return ""
  if (integer) return Math.trunc(n).toLocaleString()
  if (precision != null) return n.toFixed(precision)
  return n.toLocaleString()
}

const VectorRenderer: FieldRenderer<VectorFieldDef> = ({
  field,
  value,
  disabled,
  onChange,
}) => {
  const arity = field.components.length
  const values = toNumberArray(value, arity)
  const isInt = Boolean(field.integer)

  if (!onChange) {
    return (
      <FieldShell label={field.label ?? field.id}>
        <div
          className="grid gap-x-3 gap-y-1"
          style={{ gridTemplateColumns: `repeat(${arity}, minmax(0, 1fr))` }}
        >
          {field.components.map((c, i) => (
            <div key={i} className="flex flex-col gap-0.5 min-w-0">
              {c.label && (
                <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
                  {c.label}
                </span>
              )}
              <div className="text-xs text-foreground tabular-nums truncate">
                {values[i] == null ? (
                  <Empty />
                ) : (
                  <>
                    {formatComponent(values[i], isInt, field.precision)}
                    {c.suffix && (
                      <span className="text-muted-foreground">{c.suffix}</span>
                    )}
                  </>
                )}
              </div>
            </div>
          ))}
        </div>
      </FieldShell>
    )
  }

  return (
    <FieldShell label={field.label ?? field.id}>
      <div
        className="grid gap-x-2"
        style={{ gridTemplateColumns: `repeat(${arity}, minmax(0, 1fr))` }}
      >
        {field.components.map((c, i) => (
          <div key={i} className="flex flex-col gap-0.5 min-w-0">
            {c.label && (
              <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
                {c.label}
              </span>
            )}
            <Input
              type="number"
              aria-label={c.label ?? `${field.id}[${i}]`}
              value={values[i] != null ? String(values[i]) : ""}
              min={c.min}
              max={c.max}
              step={c.step ?? (isInt ? 1 : 0.01)}
              disabled={disabled}
              onChange={(e) => {
                const raw = e.target.value
                const parsed = raw === "" ? null : Number(raw)
                const next = values.slice()
                next[i] =
                  parsed != null && Number.isFinite(parsed)
                    ? isInt
                      ? Math.trunc(parsed)
                      : parsed
                    : null
                onChange(next)
              }}
            />
          </div>
        ))}
      </div>
    </FieldShell>
  )
}

export function registerBuiltinRenderers(): void {
  registerFieldRenderer(
    "text",
    TextRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "textarea",
    TextareaRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "number",
    NumberRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "slider",
    SliderRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "select",
    SelectRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "color",
    ColorRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "checkbox",
    CheckboxRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "file",
    FileRenderer as FieldRenderer<import("./types").FieldDef>,
  )
  registerFieldRenderer(
    "vector",
    VectorRenderer as FieldRenderer<import("./types").FieldDef>,
  )
}

export type {
  FieldRenderer,
  FieldRendererProps,
}
