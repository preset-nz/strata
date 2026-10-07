import { useQuery } from "@tanstack/react-query"
import { type BatchSummary, getBatch } from "@/features/library/api"

export function useBatchDetails(id: string | null): BatchSummary | null {
  const { data } = useQuery({
    queryKey: ["batch", id],
    queryFn: () => getBatch(id!),
    enabled: id !== null,
    staleTime: 30_000,
  })
  return data ?? null
}
