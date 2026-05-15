import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { ActressItem, ActressGroupItem } from '@/types'

export const useActressStore = defineStore('actress', {
  state: () => ({
    actresses: [] as ActressItem[],
    total: 0,
    loading: false,
    byLetter: [] as { letter: string; actresses: ActressItem[] }[],
    groups: [] as ActressGroupItem[],
  }),
  actions: {
    async fetchByLetter() {
      this.loading = true
      try {
        this.byLetter = await invoke('get_actresses_by_letter')
      } finally {
        this.loading = false
      }
    },
    async fetchPaginated(page = 1, pageSize = 20, search?: string, sortField?: string, sortOrder?: string) {
      this.loading = true
      try {
        const result: any = await invoke('get_actresses_paginated', {
          page, pageSize, search, sortField, sortOrder,
        })
        this.actresses = result.list || []
        this.total = result.total || 0
      } finally {
        this.loading = false
      }
    },
    async fetchGroups() {
      this.groups = await invoke('get_actress_groups')
    },
    async syncData() {
      await invoke('sync_actress_data')
    },
    async updateActress(id: number, data: Partial<ActressItem>) {
      await invoke('update_actress', { id, data })
    },
    async deleteActresses(ids: number[]): Promise<number> {
      return await invoke('delete_actresses', { ids })
    },
    async getAliases(actressId: number): Promise<string[]> {
      return await invoke('get_actress_aliases', { actressId })
    },
    async addAlias(actressId: number, alias: string) {
      await invoke('add_actress_alias', { actressId, alias })
    },
    async mergeActresses(sourceId: number, targetId: number) {
      await invoke('merge_actresses', { sourceId, targetId })
    },
  },
})
