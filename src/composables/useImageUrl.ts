import { invoke } from '@tauri-apps/api/core'

// Cache of path -> data URL
const cache = new Map<string, string>()

export async function imageUrl(path: string | null | undefined): Promise<string> {
  if (!path) return ''
  if (cache.has(path)) return cache.get(path)!
  try {
    const url: string = await invoke('read_image_base64', { path: path.replace(/\\/g, '/') })
    cache.set(path, url)
    return url
  } catch {
    return ''
  }
}
