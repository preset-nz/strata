import { useMemo } from "react"
import { Separator } from "@/components/ui/separator"
import { getFieldRenderer, getScope } from "./registry"
import type {
  FieldDef,
  PropertyGroupDef,
  ScopeContext,
} from "./types"

interface PropertyPanelProps {
  scopeKey: string
  selection: unknown
  ctx: ScopeContext
  readOnly?: boolean
  emptyState?: React.ReactNode
}

export function PropertyPanel({
  scopeKey,
  selection,
  ctx,
  readOnly = false,
  emptyState,
}: PropertyPanelProps) {
  const scope = getScope(scopeKey)

  const values = useMemo(() => {
    if (!scope) return {} as Record<string, unknown>
    try {
      return scope.read(selection, ctx) as Record<string, unknown>
    } catch (e) {
      console.error("[properties] scope read failed:", scopeKey, e)
      return {} as Record<string, unknown>
    }
  }, [scope, scopeKey, selection, ctx])

  if (!scope) return <>{emptyState ?? null}</>

  const onChange = readOnly || !scope.write
    ? undefined
    : (path: string, val: unknown) => scope.write!(path, val, selection, ctx)

  return (
    <div className="flex flex-col">
      {scope.schema.groups.map((group, idx) => (
        <PropertyGroup
          key={group.id}
          group={group}
          values={values}
          ctx={ctx}
          onChange={onChange}
          isLast={idx === scope.schema.groups.length - 1}
        />
      ))}
    </div>
  )
}

function PropertyGroup({
  group,
  values,
  ctx,
  onChange,
  isLast,
}: {
  group: PropertyGroupDef
  values: Record<string, unknown>
  ctx: ScopeContext
  onChange?: (path: string, val: unknown) => void
  isLast: boolean
}) {
  return (
    <section className="flex flex-col gap-3 px-3 py-3">
      <header>
        <h3 className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          {group.title}
        </h3>
        {group.description && (
          <p className="mt-1 text-xs text-muted-foreground">
            {group.description}
          </p>
        )}
      </header>
      <div className="flex flex-col gap-3">
        {group.rows.map((row, idx) => {
          const fields = Array.isArray(row) ? row : [row]
          return (
            <div
              key={idx}
              className={
                fields.length > 1 ? "grid grid-cols-2 gap-2" : undefined
              }
            >
              {fields.map((f) => (
                <FieldSlot
                  key={f.id}
                  field={f}
                  values={values}
                  ctx={ctx}
                  onChange={onChange}
                />
              ))}
            </div>
          )
        })}
      </div>
      {!isLast && <Separator className="mt-1" />}
    </section>
  )
}

function FieldSlot({
  field,
  values,
  ctx,
  onChange,
}: {
  field: FieldDef
  values: Record<string, unknown>
  ctx: ScopeContext
  onChange?: (path: string, val: unknown) => void
}) {
  const renderer = getFieldRenderer(field.kind)
  if (!renderer) {
    console.warn("[properties] no renderer for kind:", field.kind)
    return null
  }
  const disabled = (() => {
    const cond = field.disabledWhen
    if (!cond) return false
    const v = values[cond.path]
    if (Object.prototype.hasOwnProperty.call(cond, "equals"))
      return v === cond.equals
    if (Object.prototype.hasOwnProperty.call(cond, "notEquals"))
      return v !== cond.notEquals
    return false
  })()

  return renderer({
    field,
    value: values[field.path],
    disabled,
    onChange: onChange ? (val) => onChange(field.path, val) : undefined,
    ctx,
  })
}
