import { convertFileSrc } from '@tauri-apps/api/core'

/**
 * Convert a local file path to a Tauri asset:// URL.
 * Uses Tauri's built-in asset protocol (no IPC base64 round-trip).
 * Handles Windows backslash paths automatically.
 */
export function imageUrl(path: string | null | undefined): string {
  if (!path) return ''
  // normalize backslashes for Windows paths
  return convertFileSrc(path.replace(/\\/g, '/'))
}
