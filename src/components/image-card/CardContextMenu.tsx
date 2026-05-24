import { ContextMenu } from "@base-ui/react/context-menu"
import { cn } from "@/lib/utils"

export type CardContextMode = "library" | "trash"

type Props = {
  mode: CardContextMode
  onMoveToTrash?: () => void
  onRestore?: () => void
  onDeletePermanently?: () => void
  children: React.ReactNode
  className?: string
}

export function CardContextMenu({
  mode,
  onMoveToTrash,
  onRestore,
  onDeletePermanently,
  children,
  className,
}: Props) {
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger className={cn("contents", className)}>
        {children}
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Positioner className="outline-none">
          <ContextMenu.Popup
            className={cn(
              "z-50 min-w-44 rounded-sm border border-border bg-popover p-1 text-xs shadow-md outline-none",
            )}
          >
            {mode === "library" && (
              <ContextMenu.Item
                onClick={() => onMoveToTrash?.()}
                className={menuItem}
              >
                Move to Trash
              </ContextMenu.Item>
            )}
            {mode === "trash" && (
              <>
                <ContextMenu.Item
                  onClick={() => onRestore?.()}
                  className={menuItem}
                >
                  Restore
                </ContextMenu.Item>
                <ContextMenu.Item
                  onClick={() => onDeletePermanently?.()}
                  className={cn(menuItem, "text-destructive")}
                >
                  Delete permanently
                </ContextMenu.Item>
              </>
            )}
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  )
}

const menuItem = cn(
  "flex w-full cursor-default select-none items-center rounded-xs px-2 py-1 text-xs outline-none",
  "data-[highlighted]:bg-muted data-[highlighted]:text-foreground",
)
