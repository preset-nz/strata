import { invoke } from "@tauri-apps/api/core"
import type { SavedQuery } from "./query"

export type SavedSearch = {
  id: string
  name: string
  /** As stored; read it through `parseQuery`. */
  query: unknown
  created_at: string
  updated_at: string
  /** The project it belongs to; null is global. */
  project_key: string | null
}

export function listSavedSearches(): Promise<SavedSearch[]> {
  return invoke<SavedSearch[]>("saved_search_list")
}

/** Saved while a project is the filter, the search belongs to that project. */
export function saveSearch(
  name: string,
  query: SavedQuery
): Promise<SavedSearch> {
  return invoke<SavedSearch>("saved_search_put", {
    name,
    query,
    projectKey: query.projectKey,
  })
}

/** Puts a deleted search back as it was, for Undo. */
export function restoreSearch(s: SavedSearch): Promise<SavedSearch> {
  return invoke<SavedSearch>("saved_search_put", {
    id: s.id,
    name: s.name,
    query: s.query,
    createdAt: s.created_at,
    projectKey: s.project_key,
  })
}

export function deleteSearch(id: string): Promise<SavedSearch | null> {
  return invoke<SavedSearch | null>("saved_search_delete", { id })
}
