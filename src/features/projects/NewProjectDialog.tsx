import { useEffect, useRef, useState } from "react"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { registerOverlay } from "@/lib/overlay"

type Props = {
  /** Where it will be made, for the hint. */
  root: string
  onCreate: (name: string, description: string) => void
  onCancel: () => void
}

/** File › New Project…: a name (which becomes the folder and the key) and a description. */
export function NewProjectDialog({ root, onCreate, onCancel }: Props) {
  const [name, setName] = useState("")
  const [description, setDescription] = useState("")
  const nameRef = useRef<HTMLInputElement>(null)
  useEffect(() => registerOverlay(), [])
  useEffect(() => nameRef.current?.focus(), [])
  const submit = () => {
    if (name.trim()) onCreate(name.trim(), description.trim())
  }
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="New Project"
      className="fixed inset-0 z-50 flex items-start justify-center bg-background/60 pt-24 backdrop-blur-sm"
      onClick={onCancel}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation()
          onCancel()
        }
      }}
    >
      <form
        onClick={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault()
          submit()
        }}
        className="flex w-96 flex-col gap-3 rounded-md border border-border bg-card p-4 shadow-xl"
      >
        <h2 className="font-semibold text-sm">New Project</h2>
        <label className="flex flex-col gap-1 text-xs">
          Name
          <Input
            ref={nameRef}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Night Drive"
          />
        </label>
        <label className="flex flex-col gap-1 text-xs">
          Description
          <Input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Optional"
          />
        </label>
        <p className="text-[10px] text-muted-foreground">
          Makes{" "}
          <span className="select-text">
            {root}/{name.trim() || "…"}
          </span>{" "}
          with its project.preset. The apps add their folders when they first
          save into it.
        </p>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" size="sm" onClick={onCancel}>
            Cancel
          </Button>
          <Button type="submit" size="sm" disabled={!name.trim()}>
            Create
          </Button>
        </div>
      </form>
    </div>
  )
}
