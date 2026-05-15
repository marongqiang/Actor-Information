import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { Task } from '@/types'

export const useTaskStore = defineStore('task', {
  state: () => ({
    tasks: [] as Task[],
  }),
  actions: {
    async fetchPendingTasks() {
      this.tasks = await invoke('get_pending_tasks')
    },
    async resumeTask(taskId: string) {
      await invoke('resume_task', { taskId })
      await this.fetchPendingTasks()
    },
    async cancelTask(taskId: string) {
      await invoke('cancel_task', { taskId })
      await this.fetchPendingTasks()
    },
  },
})
