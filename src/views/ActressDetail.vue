<template>
  <div class="actress-detail" v-if="actress">
    <div class="detail-header">
      <el-button size="small" @click="$router.back()">← 返回</el-button>
    </div>
    <div class="detail-content">
      <div class="avatar-col">
        <img v-if="actress.avatar_local" :src="assetUrl(actress.avatar_local)" class="avatar-large" />
        <div v-else class="avatar-placeholder"><el-icon :size="48"><UserFilled /></el-icon></div>
        <h2>{{ actress.name }}</h2>
        <div class="meta">
          <el-tag v-if="actress.debut_year" size="small">{{ actress.debut_year }}年出道</el-tag>
          <el-tag v-if="actress.height" size="small">{{ actress.height }}cm</el-tag>
          <el-tag v-if="actress.cup" size="small">{{ actress.cup }}</el-tag>
          <el-tag v-if="actress.bust" size="small">{{ actress.bust }}/{{ actress.waist }}/{{ actress.hip }}</el-tag>
        </div>
      </div>
      <div class="info-col">
        <div v-if="(actress as any)._aliases?.length" class="section">
          <h3>别名</h3>
          <el-tag v-for="a in (actress as any)._aliases" :key="a" size="small" style="margin:2px;">{{ a }}</el-tag>
        </div>
        <div class="section">
          <h3>关联影片 ({{ movies.length }})</h3>
          <div class="movie-grid" v-if="movies.length">
            <div v-for="m in movies" :key="m.file_id" class="movie-card" @click="$router.push(`/detail/${m.file_id}`)">
              <div class="poster-container">
                <img v-if="m.poster_local" :src="assetUrl(m.poster_local)" class="poster-img" />
                <div v-else class="poster-placeholder"><el-icon :size="28"><VideoCamera /></el-icon></div>
              </div>
              <p class="movie-title">{{ m.title }}</p>
            </div>
          </div>
          <p v-else style="color: #888;">暂无关联影片</p>
        </div>
        <div v-if="actress.local_folder_name" class="section">
          <h3>本地目录</h3>
          <p style="color: #888; font-size: 12px;">{{ actress.local_folder_name }}</p>
        </div>
      </div>
    </div>
  </div>
  <div v-else class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { Loading, UserFilled, VideoCamera } from '@element-plus/icons-vue'
import type { ActressItem } from '@/types'

const route = useRoute()
const actress = ref<ActressItem | null>(null)
const movies = ref<any[]>([])

function assetUrl(p: string) { return convertFileSrc(p.replace(/\\/g, '/')) }

onMounted(async () => {
  const id = Number(route.params.id)
  const result: any = await invoke('get_actresses_paginated', { page: 1, pageSize: 1, search: String(id) })
  // Find by id - we need a better way but for now use the store
  try {
    const data: any = await invoke('find_actress', { name: '' })
    // Use the actress store to get by id
    const list: any = await invoke('get_actresses_paginated', { page: 1, pageSize: 200 })
    const found = list?.list?.find((a: any) => a.id === id)
    if (found) {
      actress.value = found
      // Get aliases
      try { (actress.value as any)._aliases = await invoke('get_actress_aliases', { actressId: id }) } catch { (actress.value as any)._aliases = [] }
      // Get related movies
      if (found.name) {
        const movieResult: any = await invoke('get_movies', { filters: {}, sort: 'title_asc', page: 1 })
        movies.value = (movieResult?.movies || []).filter((m: any) => {
          // Simple match by actor name in movie title or genre
          return m.title?.includes(found.name) || false
        }).slice(0, 30)
      }
    }
  } catch (e) { console.error(e) }
})
</script>

<style scoped>
.actress-detail { max-width: 1000px; }
.detail-header { margin-bottom: 16px; }
.detail-content { display: flex; gap: 24px; }
.avatar-col { flex-shrink: 0; text-align: center; }
.avatar-large { width: 160px; height: 160px; border-radius: 50%; object-fit: cover; border: 3px solid #3a3a5a; }
.avatar-placeholder { width: 160px; height: 160px; border-radius: 50%; background: #2a2a4a; display: flex; align-items: center; justify-content: center; color: #555; margin: 0 auto; }
.avatar-col h2 { font-size: 20px; margin: 12px 0 8px; }
.meta { display: flex; gap: 6px; flex-wrap: wrap; justify-content: center; }
.info-col { flex: 1; }
.section { margin-bottom: 20px; }
.section h3 { font-size: 15px; margin-bottom: 10px; color: #e0e0e0; }
.movie-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 10px; }
.movie-card { cursor: pointer; border-radius: 6px; overflow: hidden; background: #1a1a2e; transition: transform 0.15s; }
.movie-card:hover { transform: scale(1.03); }
.poster-container { aspect-ratio: 2/3; background: #252540; display: flex; align-items: center; justify-content: center; }
.poster-img { width: 100%; height: 100%; object-fit: cover; }
.poster-placeholder { color: #555; }
.movie-title { font-size: 11px; padding: 4px 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: #c0c0d0; }
.loading { text-align: center; padding: 80px; color: #888; }
</style>
