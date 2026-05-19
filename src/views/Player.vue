<template>
  <div class="player-page">
    <div class="player-header">
      <el-button @click="$router.back()">
        <el-icon><ArrowLeft /></el-icon> 返回
      </el-button>
      <h2>{{ movie?.title || '播放中...' }}</h2>
      <span v-if="progress > 0" class="progress-text">
        进度: {{ formatTime(progress) }} / {{ formatTime(duration) }}
      </span>
    </div>
    <div class="player-container" v-if="playUrl">
      <video
        ref="videoRef"
        :src="playUrl"
        controls
        autoplay
        style="width: 100%; max-height: 70vh; background: #000; border-radius: 8px;"
        @timeupdate="onTimeUpdate"
        @ended="onEnded"
      />
    </div>
    <div v-else class="loading">
      <el-icon class="is-loading"><Loading /></el-icon> 获取播放链接中...
    </div>
    <div class="playback-controls">
      <el-button @click="markWatched" v-if="!isFinished">标记已看完</el-button>
      <el-button @click="markUnwatched" v-if="isFinished">标记未看完</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import type { MovieDetail } from '@/types'
import { Loading, ArrowLeft } from '@element-plus/icons-vue'
import { formatTime } from '@/composables/useFormat'

const route = useRoute()
const router = useRouter()
const movie = ref<MovieDetail | null>(null)
const playUrl = ref('')
const progress = ref(0)
const duration = ref(0)
const isFinished = ref(false)
const videoRef = ref<HTMLVideoElement | null>(null)
let saveTimer: ReturnType<typeof setInterval> | null = null

async function onTimeUpdate() {
  if (!videoRef.value) return
  progress.value = Math.floor(videoRef.value.currentTime)
  duration.value = Math.floor(videoRef.value.duration) || 0
}

async function saveProgress() {
  if (!movie.value || duration.value === 0) return
  await invoke('save_progress', {
    fileId: movie.value.file_id,
    progress: progress.value,
    duration: duration.value,
  })
}

async function onEnded() {
  isFinished.value = true
  if (movie.value) {
    try { await invoke('end_playback', { fileId: movie.value.file_id }) } catch { /* ignore */ }
  }
}

async function markWatched() {
  if (!movie.value) return
  await invoke('end_playback', { fileId: movie.value.file_id })
  isFinished.value = true
}

async function markUnwatched() {
  if (!movie.value) return
  await invoke('save_progress', { fileId: movie.value.file_id, progress: 0, duration: 0 })
  isFinished.value = false
  progress.value = 0
}

onMounted(async () => {
  const fileId = route.params.fileId as string
  try {
    movie.value = await invoke('get_movie_detail', { fileId })
    const result: any = await invoke('get_play_url', { fileId })
    playUrl.value = result.url

    const prog: any = await invoke('get_progress', { fileId })
    progress.value = prog.progress || 0
    duration.value = prog.duration || 0
    isFinished.value = !!prog.is_finished
  } catch (e) {
    console.error('Failed to load player:', e)
  }
  saveTimer = setInterval(saveProgress, 5000)
})

onUnmounted(() => {
  if (saveTimer) clearInterval(saveTimer)
  saveProgress()
})
</script>

<style scoped>
.player-page { max-width: 1000px; margin: 0 auto; }
.player-header {
  display: flex; align-items: center; gap: 16px;
  margin-bottom: 20px; padding: 12px; background: #1a1a2e; border-radius: 8px;
}
.player-header h2 { flex: 1; font-size: 18px; }
.progress-text { color: #888; font-size: 13px; }
.player-container { margin-bottom: 16px; }
.loading { text-align: center; padding: 80px; color: #888; }
.playback-controls { display: flex; gap: 10px; justify-content: center; padding: 12px; }
</style>
