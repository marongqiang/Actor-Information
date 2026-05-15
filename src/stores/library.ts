import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { MovieItem, MovieDetail, FilterParams, GroupItem } from '@/types'

export const useLibraryStore = defineStore('library', {
  state: () => ({
    movies: [] as MovieItem[],
    total: 0,
    loading: false,
    groups: [] as GroupItem[],
    currentPage: 1,
    filters: {} as FilterParams,
    sort: 'updated_at_desc',
  }),
  actions: {
    async fetchMovies(page = 1, pageSize = 20) {
      this.loading = true
      try {
        const result: any = await invoke('get_movies', {
          filters: this.filters,
          sort: this.sort,
          page,
          pageSize,
        })
        this.movies = result.movies || []
        this.total = result.total || 0
        this.currentPage = page
      } catch(e: any) {
        console.error('fetchMovies ERROR:', e)
      } finally {
        this.loading = false
      }
    },
    async fetchMovieDetail(fileId: string): Promise<MovieDetail | null> {
      return await invoke('get_movie_detail', { fileId })
    },
    async fetchGroups() {
      this.groups = await invoke('get_groups')
    },
    async setFilters(filters: FilterParams) {
      this.filters = filters
      await this.fetchMovies(1)
    },
    async toggleHidden(fileIds: string[]) {
      await invoke('hide_movies', { fileIds })
      await this.fetchMovies(this.currentPage)
    },
    async unhideMovies(fileIds: string[]) {
      await invoke('unhide_movies', { fileIds })
      await this.fetchMovies(this.currentPage)
    },
    async batchAction(fileIds: string[], action: string) {
      await invoke('batch_action', { fileIds, action })
      await this.fetchMovies(this.currentPage)
    },
  },
})
