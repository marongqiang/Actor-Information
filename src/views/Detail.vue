<template>
  <div class="detail-page" v-if="movie">
    <div class="backdrop" v-if="movie.backdrop_local">
      <img :src="assetUrl(movie.backdrop_local)" alt="" />
    </div>
    <div class="detail-content">
      <div class="poster-col">
        <img v-if="movie.poster_local" :src="assetUrl(movie.poster_local)" class="detail-poster" />
        <div v-else class="detail-poster placeholder"><el-icon :size="64"><PictureFilled /></el-icon></div>
      </div>
      <div class="info-col">
        <h1>{{ movie.title }}</h1>
        <p v-if="movie.original_title" class="original-title">{{ movie.original_title }}</p>
        <div class="meta-tags">
          <el-tag v-if="movie.year">{{ movie.year }}</el-tag>
          <el-tag v-if="movie.rating" type="warning">★ {{ movie.rating.toFixed(1) }}</el-tag>
          <el-tag v-if="movie.runtime">{{ movie.runtime }} 分钟</el-tag>
          <el-tag v-for="g in movie.genre" :key="g" type="info">{{ g }}</el-tag>
        </div>
        <p class="overview" v-if="movie.overview">{{ movie.overview }}</p>
        <div class="detail-meta">
          <p v-if="movie.director"><strong>导演：</strong>{{ movie.director }}</p>
          <p v-if="movie.actors?.length"><strong>演员：</strong>
            <span v-for="(name, i) in movie.actors" :key="name">
              <a class="actor-link" @click.stop="goActress(name)">{{ name }}</a>
              <span v-if="i < movie.actors.length - 1"> / </span>
            </span>
          </p>
          <p><strong>文件：</strong>{{ movie.file_name }}</p>
          <p><strong>大小：</strong>{{ formatSize(movie.file_size) }}</p>
        </div>
        <div class="actions">
          <el-button type="primary" @click="$router.push(`/player/${movie.file_id}`)">
            <el-icon><VideoPlay /></el-icon> 播放
          </el-button>
          <el-button type="success" @click="playExternal">
            <el-icon><Connection /></el-icon> 外部播放
          </el-button>
          <el-button @click="toggleHidden">
            {{ movie.is_hidden ? '取消隐藏' : '隐藏' }}
          </el-button>
        </div>
        <div class="groups-section" v-if="groups.length">
          <p><strong>所属分组：</strong></p>
          <el-tag v-for="g in groups" :key="g.id" type="success" style="margin: 2px">{{ g.name }}</el-tag>
        </div>
      </div>
    </div>
  </div>
  <div v-else class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import type { MovieDetail, GroupItem } from '@/types'
import { Loading, PictureFilled, VideoPlay, Connection } from '@element-plus/icons-vue'

const route = useRoute()
const router = useRouter()
const movie = ref<MovieDetail | null>(null)
const groups = ref<GroupItem[]>([])

function assetUrl(path: string) {
  return convertFileSrc(path)
}

function formatSize(bytes: number) {
  if (!bytes) return '未知'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  let size = bytes
  while (size > 1024 && i < units.length - 1) { size /= 1024; i++ }
  return size.toFixed(1) + ' ' + units[i]
}

async function toggleHidden() {
  if (!movie.value) return
  if (movie.value.is_hidden) {
    await invoke('unhide_movies', { fileIds: [movie.value.file_id] })
  } else {
    await invoke('hide_movies', { fileIds: [movie.value.file_id] })
  }
  movie.value.is_hidden = !movie.value.is_hidden
}

async function goActress(name: string) {
  const found: any = await invoke('find_actress', { name })
  if (found?.id) { router.push(`/actress/${found.id}`) }
  else { ElMessage.info(`未找到演员: ${name}`) }
}

async function playExternal() {
  if (!movie.value) return
  const result: any = await invoke('get_play_url', { fileId: movie.value.file_id })
  const extPlayer: string | null = await invoke('get_config', { key: 'external_player' })
  if (extPlayer && result.url) {
    const { Command } = await import('@tauri-apps/plugin-shell')
    await Command.create(extPlayer, [result.url]).execute()
  }
}

onMounted(async () => {
  const fileId = route.params.fileId as string
  movie.value = await invoke('get_movie_detail', { fileId })
  groups.value = movie.value?.groups || []
})
</script>

<style scoped>
.detail-page { }
.backdrop { height: 240px; overflow: hidden; border-radius: 8px; margin-bottom: 20px; }
.backdrop img { width: 100%; height: 100%; object-fit: cover; }
.detail-content { display: flex; gap: 24px; }
.poster-col { flex-shrink: 0; }
.detail-poster { width: 240px; border-radius: 8px; }
.detail-poster.placeholder {
  aspect-ratio: 2/3; background: #2a2a4a;
  display: flex; align-items: center; justify-content: center; color: #555;
}
.info-col { flex: 1; }
.info-col h1 { font-size: 24px; margin-bottom: 8px; }
.original-title { color: #888; margin-bottom: 12px; }
.meta-tags { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 16px; }
.overview { color: #bbb; line-height: 1.6; margin-bottom: 16px; }
.detail-meta p { margin: 6px 0; color: #999; }
.actions { display: flex; gap: 10px; margin-top: 20px; }
.groups-section { margin-top: 16px; }
.loading { text-align: center; padding: 60px; color: #888; }
</style>
