import { useQuery } from "@tanstack/react-query"
import { getImageDetails, type ImageDetails } from "@/features/contact-sheet/api"

export function useImageDetails(id: string | null): ImageDetails | null {
  const { data } = useQuery({
    queryKey: ["image-details", id],
    queryFn: () => getImageDetails(id!),
    enabled: id !== null,
    staleTime: 30_000,
  })
  return data ?? null
}
