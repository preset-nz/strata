import { ContextMenu } from "@base-ui/react/context-menu"
import { cn } from "@/lib/utils"

/** What the menu offers for collections; absent, it offers nothing. */
export type CollectionMenu = {
  collections: { id: string; name: string }[]
  onAdd: (collectionId: string) => void
  onNew: () => void
  /** Set while the library shows one collection. */
  onRemove?: () => void
}

export type CardContextMode = "library" | "trash"

type Props = {
  mode: CardContextMode
  onMoveToTrash?: () => void
  onRestore?: () => void
  onDeletePermanently?: () => void
  collections?: CollectionMenu
  children: React.ReactNode
  className?: string
}

export function CardContextMenu({
  mode,
  onMoveToTrash,
  onRestore,
  onDeletePermanently,
  collections,
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
          <ContextMenu.Popup className={popup}>
            {mode === "library" && collections && (
              <>
                <ContextMenu.SubmenuRoot>
                  <ContextMenu.SubmenuTrigger className={cn(menuItem, "justify-between")}>
                    Add to Collection <span aria-hidden>›</span>
                  </ContextMenu.SubmenuTrigger>
                  <ContextMenu.Portal>
                    <ContextMenu.Positioner className="outline-none" sideOffset={4}>
                      <ContextMenu.Popup className={popup}>
                        {collections.collections.map((c) => (
                          <ContextMenu.Item key={c.id} onClick={() => collections.onAdd(c.id)} className={menuItem}>
                            {c.name}
                          </ContextMenu.Item>
                        ))}
                        {collections.collections.length > 0 && <ContextMenu.Separator className={separator} />}
                        <ContextMenu.Item onClick={collections.onNew} className={menuItem}>
                          New Collection…
                        </ContextMenu.Item>
                      </ContextMenu.Popup>
                    </ContextMenu.Positioner>
                  </ContextMenu.Portal>
                </ContextMenu.SubmenuRoot>
                {collections.onRemove && (
                  <ContextMenu.Item onClick={collections.onRemove} className={menuItem}>
                    Remove from Collection
                  </ContextMenu.Item>
                )}
                <ContextMenu.Separator className={separator} />
              </>
            )}
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

const popup = "z-50 min-w-44 rounded-sm border border-border bg-popover p-1 text-xs shadow-md outline-none"
const separator = "my-1 h-px bg-border"

const menuItem = cn(
  "flex w-full cursor-default select-none items-center rounded-xs px-2 py-1 text-xs outline-none",
  "data-[highlighted]:bg-muted data-[highlighted]:text-foreground",
)
