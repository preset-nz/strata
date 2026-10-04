import { invoke } from "@tauri-apps/api/core"
import type { SavedQuery } from "./query"

export type SavedSearch = {
  id: string
  name: string
  /** As stored; read it through `parseQuery`. */
  query: unknown
  created_at: string
  updated_at: string
}

export function listSavedSearches(): Promise<SavedSearch[]> {
  return invoke<SavedSearch[]>("saved_search_list")
}

export function saveSearch(name: string, query: SavedQuery): Promise<SavedSearch> {
  return invoke<SavedSearch>("saved_search_put", { name, query })
}

/** Puts a deleted search back as it was, for Undo. */
export function restoreSearch(s: SavedSearch): Promise<SavedSearch> {
  return invoke<SavedSearch>("saved_search_put", {
    id: s.id,
    name: s.name,
    query: s.query,
    createdAt: s.created_at,
  })
}

export function deleteSearch(id: string): Promise<SavedSearch | null> {
  return invoke<SavedSearch | null>("saved_search_delete", { id })
}
