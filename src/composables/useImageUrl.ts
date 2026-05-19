import { invoke } from '@tauri-apps/api/core'
import { shallowRef, type Ref } from 'vue'

const SFW_PLACEHOLDER = 'D:/Media Library/src-tauri/target/release/data/test.png'
const cache = new Map<string, string>()
const pending = new Map<string, Promise<string>>()
let sfwMode = false
let sfwLoaded = false

async function checkSfw(): Promise<boolean> {
  if (!sfwLoaded) {
    sfwMode = (await invoke('get_config', { key: 'sfw_mode' }) as string) === 'true'
    sfwLoaded = true
  }
  return sfwMode
}

/** Called when SFW mode toggle changes — clear cache so images reload */
export function resetSfwMode() {
  sfwLoaded = false
  cache.clear()
}

/**
 * Load a local image file and return as base64 data URL.
 * Deduplicates concurrent requests and caches results in memory.
 */
export async function imageUrl(path: string | null | undefined): Promise<string> {
  if (!path) return ''
  // SFW mode: always return placeholder
  if (await checkSfw()) { path = SFW_PLACEHOLDER }
  const normalized = path.replace(/\\/g, '/')
  if (cache.has(normalized)) return cache.get(normalized)!

  if (pending.has(normalized)) return pending.get(normalized)!

  const promise = (async () => {
    try {
      const url: string = await invoke('read_image_base64', { path: normalized })
      cache.set(normalized, url)
      return url
    } catch {
      cache.set(normalized, '')
      return ''
    }
  })()

  pending.set(normalized, promise)
  const result = await promise
  pending.delete(normalized)
  return result
}

/**
 * Reactive composable: returns a ref<string> that auto-resolves to the image URL.
 * Usage: const src = useImageUrl(movie.poster_local)
 *        <img :src="src" />
 */
export function useImageUrl(path: string | null | undefined): Ref<string> {
  const src = shallowRef('')
  if (path) {
    imageUrl(path).then(v => { src.value = v })
  }
  return src
}

/** Synchronously get a URL from cache only (no fetch). Returns '' if not cached. */
export function imageUrlCached(path: string | null | undefined): string {
  if (!path) return ''
  return cache.get(path.replace(/\\/g, '/')) || ''
}

/** Preload multiple image paths in parallel. Returns when all are cached. */
export async function preloadImages(paths: (string | null | undefined)[]): Promise<void> {
  const valid = paths.filter(Boolean).map(p => imageUrl(p!))
  await Promise.allSettled(valid)
}
