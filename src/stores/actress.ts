import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { ActressItem, ActressGroupItem, MergeOptions, MergeResult, DuplicatePair } from '@/types'

export const useActressStore = defineStore('actress', {
  state: () => ({
    actresses: [] as ActressItem[],
    total: 0,
    loading: false,
    byLetter: [] as { letter: string; actresses: ActressItem[] }[],
    tablePage: 1,
    tableSearch: '',
    tableShowPending: undefined as boolean | undefined,
    groups: [] as ActressGroupItem[],
    pendingCount: 0,
    duplicates: [] as DuplicatePair[],
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
    async fetchPaginated(page = 1, pageSize = 20, search?: string, sortField?: string, sortOrder?: string, includePending?: boolean, groupId?: number) {
      this.loading = true
      try {
        const result: any = await invoke('get_actresses_paginated', {
          page, pageSize, search: search || null, sortField, sortOrder, includePending, groupId,
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

    // 1.2.0 新增

    async mergeActresses(sourceId: number, targetId: number): Promise<MergeResult> {
      return await invoke('merge_actresses', {
        sourceId, targetId,
        options: { mergeFolders: true, conflictPolicy: 'rename', dryRun: true },
      })
    },

    async mergeActressesWithOptions(sourceId: number, targetId: number, options: MergeOptions): Promise<MergeResult> {
      return await invoke('merge_actresses', { sourceId, targetId, options })
    },

    async scanLocalFolder(folderPath?: string): Promise<{ added: number; total: number }> {
      return await invoke('scan_local_actress_folder', { folderPath })
    },

    async refreshAvatar(actressId: number) {
      await invoke('refresh_actress_avatar', { actressId })
    },

    async confirmActor(actorId: number, accepted: boolean) {
      await invoke('confirm_actor', { actorId, accepted })
    },

    async updateActorLocalFolder(actorId: number, folderPath: string | null) {
      await invoke('update_actor_local_folder', { actorId, folderPath })
    },

    async renameActorAndFolder(actorId: number, newName: string, renameFolder: boolean) {
      return await invoke('rename_actor_and_folder', { actorId, newName, renameFolder })
    },

    async detectDuplicates(threshold?: number): Promise<DuplicatePair[]> {
      this.duplicates = await invoke('detect_duplicate_actresses', { threshold })
      return this.duplicates
    },

    async getLocalFolder(actressId: number): Promise<string | null> {
      return await invoke('get_actress_local_folder', { actressId })
    },

    async syncWithLocalFolder(actressId: number) {
      await invoke('sync_actress_with_local_folder', { actressId })
    },
  },
})
