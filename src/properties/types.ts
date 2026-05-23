// Built-in kinds shipped with the package. Apps may register additional kinds
// via `registerFieldRenderer(kind, Component)` — the panel looks up by string.
export type BuiltinFieldKind =
  | "text"
  | "textarea"
  | "number"
  | "slider"
  | "select"
  | "color"
  | "checkbox"
  | "file"

export interface BaseField {
  kind: string
  id: string
  label?: string
  path: string
  disabledWhen?: { path: string; equals?: unknown; notEquals?: unknown }
}

// A custom field is any BaseField whose `kind` is not one of the built-ins.
// The host's registered renderer for that kind decides what extra props it reads.
export type CustomFieldDef = BaseField & {
  [extra: string]: unknown
}

export interface SelectOption {
  value: string
  label: string
}

export interface SelectFieldDef extends BaseField {
  kind: "select"
  options?: SelectOption[]
  optionsProvider?: (ctx: ScopeContext) => SelectOption[]
}

export interface ColorFieldDef extends BaseField {
  kind: "color"
  presets?: string[]
}

export interface TextFieldDef extends BaseField {
  kind: "text"
  placeholder?: string
}

export interface TextareaFieldDef extends BaseField {
  kind: "textarea"
  placeholder?: string
  rows?: number
}

export interface NumberFieldDef extends BaseField {
  kind: "number"
  min?: number
  max?: number
  step?: number
}

export interface SliderFieldDef extends BaseField {
  kind: "slider"
  min?: number
  max?: number
  step?: number
}

export interface CheckboxFieldDef extends BaseField {
  kind: "checkbox"
}

// A tuple of related scalars sharing one row. Houdini-style float3 / int2 /
// stringN — variable arity, components share semantic context (LCh, RGB,
// position, dimensions, …). The path resolves to an array; each component
// describes one slot.
export interface VectorComponentDef {
  label?: string
  suffix?: string
  min?: number
  max?: number
  step?: number
}

export interface VectorFieldDef extends BaseField {
  kind: "vector"
  components: VectorComponentDef[]
  integer?: boolean
  precision?: number
}

export interface FileFieldDef extends BaseField {
  kind: "file"
  accept?: string
  helperText?: string
}

export type BuiltinFieldDef =
  | SelectFieldDef
  | ColorFieldDef
  | TextFieldDef
  | TextareaFieldDef
  | NumberFieldDef
  | SliderFieldDef
  | CheckboxFieldDef
  | FileFieldDef
  | VectorFieldDef

export type FieldDef = BuiltinFieldDef | CustomFieldDef

export interface PropertyGroupDef {
  id: string
  title: string
  rows: Array<FieldDef | FieldDef[]>
  description?: string
  collapsible?: boolean
  defaultCollapsed?: boolean
}

export interface PropertySchema {
  version: number
  groups: PropertyGroupDef[]
}

// Opaque to the package. Each host types its own context.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type ScopeContext = any

export interface Scope<S = unknown, V = Record<string, unknown>> {
  schema: PropertySchema
  read: (selection: S, ctx: ScopeContext) => V
  write?: (
    path: string,
    value: unknown,
    selection: S,
    ctx: ScopeContext,
  ) => void
}

export interface FieldRendererProps<F extends FieldDef = FieldDef> {
  field: F
  value: unknown
  disabled: boolean
  onChange?: (next: unknown) => void
  ctx: ScopeContext
}

export type FieldRenderer<F extends FieldDef = FieldDef> = React.FC<
  FieldRendererProps<F>
>

import type React from "react"
