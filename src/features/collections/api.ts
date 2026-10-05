import { invoke } from "@tauri-apps/api/core"

export type Collection = { id: string; name: string; count: number }

// Every change is one undo step in the curation history (Edit › Undo).
export const listCollections = () => invoke<Collection[]>("collection_list")
export const createCollection = (name: string, images: string[]) =>
  invoke<string>("collection_create", { name, images })
export const renameCollection = (id: string, name: string) => invoke<void>("collection_rename", { id, name })
export const deleteCollection = (id: string) => invoke<void>("collection_delete", { id })
export const addToCollection = (id: string, images: string[]) => invoke<void>("collection_add", { id, images })
export const removeFromCollection = (id: string, images: string[]) =>
  invoke<void>("collection_remove", { id, images })
