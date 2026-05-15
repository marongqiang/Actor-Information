import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { FileItem } from '@/types'

export const useScanStore = defineStore('scan', {
  state: () => ({
    roots: [] as { cid: string; name: string }[],
    files: [] as FileItem[],
    totalFiles: 0,
    scanning: false,
  }),
  actions: {
    async listRoot() {
      this.roots = await invoke('list_root')
    },
    async getFiles(path: string, page = 1, pageSize = 50) {
      const result: any = await invoke('get_files', { path, page, pageSize })
      this.files = result.files || []
      this.totalFiles = result.total || 0
    },
    async scanDirectory(path: string, depth: number, mode: string) {
      this.scanning = true
      try {
        return await invoke('scan_directory', { path, depth, mode })
      } finally {
        this.scanning = false
      }
    },
  },
})
