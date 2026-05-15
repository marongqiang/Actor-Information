import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export function usePlayerRefresh(fileId: string) {
  const playUrl = ref('')
  const expiresAt = ref(0)
  const loading = ref(false)
  const error = ref('')
  let refreshTimer: ReturnType<typeof setInterval> | null = null

  async function fetchUrl() {
    loading.value = true
    error.value = ''
    try {
      const result: any = await invoke('get_play_url', { fileId })
      playUrl.value = result.url
      expiresAt.value = result.expire_at
    } catch (e: any) {
      error.value = e.message || '获取播放链接失败'
    } finally {
      loading.value = false
    }
  }

  async function refreshUrl() {
    loading.value = true
    error.value = ''
    try {
      const result: any = await invoke('refresh_play_url', { fileId })
      playUrl.value = result.url
      expiresAt.value = result.expire_at
    } catch (e: any) {
      error.value = e.message || '续期失败'
    } finally {
      loading.value = false
    }
  }

  function startAutoRefresh(intervalMinutes = 4) {
    stopAutoRefresh()
    refreshTimer = setInterval(refreshUrl, intervalMinutes * 60 * 1000)
  }

  function stopAutoRefresh() {
    if (refreshTimer) {
      clearInterval(refreshTimer)
      refreshTimer = null
    }
  }

  onMounted(() => {
    fetchUrl()
    startAutoRefresh()
  })

  onUnmounted(() => {
    stopAutoRefresh()
  })

  return { playUrl, expiresAt, loading, error, refreshUrl, fetchUrl }
}
