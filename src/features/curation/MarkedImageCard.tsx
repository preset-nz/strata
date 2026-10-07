import type { ComponentProps } from "react"

import { ImageCard } from "@/components/image-card/ImageCard"
import { setColourLabel, setFavourite, useMark } from "./marks"

type Props = Omit<
  ComponentProps<typeof ImageCard>,
  "isFavourite" | "colourLabel" | "onFavouriteToggle" | "onColourLabelChange"
> & {
  id: string
  /** Called after a change lands, so the host can refresh counts. */
  onMarked?: () => void
}

/** An image card whose heart and label read from, and write to, the catalog. */
export function MarkedImageCard({ id, onMarked, ...props }: Props) {
  const mark = useMark(id)
  return (
    <ImageCard
      {...props}
      id={id}
      isFavourite={mark.favourite}
      colourLabel={mark.label}
      onFavouriteToggle={() =>
        void setFavourite([id], !mark.favourite).then(onMarked)
      }
      onColourLabelChange={(label) =>
        void setColourLabel([id], label).then(onMarked)
      }
    />
  )
}
