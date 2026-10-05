import { invoke } from "@tauri-apps/api/core"

/** A work project: a folder with project.preset, plus Strata's curation of it. */
export type Project = {
  key: string
  name: string
  description: string
  folder: string
  favourite: boolean
  archived: boolean
  count: number
}

export const listProjects = () => invoke<Project[]>("project_list")
/** Makes the folder and marker on disk. Not an undo step. Resolves with the key. */
export const createProject = (name: string, description: string) =>
  invoke<string>("project_create", { name, description })
export const setProjectFavourite = (key: string, on: boolean) => invoke<void>("project_set_favourite", { key, on })
export const setProjectArchived = (key: string, on: boolean) => invoke<void>("project_set_archived", { key, on })
export const addToProject = (key: string, images: string[]) => invoke<void>("project_add", { key, images })
/** False when nothing could go: the images sit under the project's folder. */
export const removeFromProject = (key: string, images: string[]) =>
  invoke<boolean>("project_remove", { key, images })
