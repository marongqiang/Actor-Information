<template>
  <div class="actress-detail" v-if="actress">
    <div class="detail-header">
      <el-button size="small" @click="$router.back()">← 返回</el-button>
    </div>
    <div class="detail-content">
      <div class="avatar-col">
        <img v-if="actress.avatar_local" :src="img" class="avatar-large" />
        <div v-else class="avatar-placeholder"><el-icon :size="48"><UserFilled /></el-icon></div>
        <h2>{{ actress.name }}</h2>
        <div class="meta">
          <el-tag v-if="actress.debut_year" size="small">{{ actress.debut_year }}年</el-tag>
          <el-tag v-if="actress.height" size="small">{{ actress.height }}cm</el-tag>
          <el-tag v-if="actress.cup" size="small">{{ actress.cup }}</el-tag>
          <span v-if="actress.bust">{{ actress.bust }}/{{ actress.waist }}/{{ actress.hip }}</span>
        </div>
      </div>
      <div class="info-col">
        <div class="section" v-if="aliases.length">
          <h3>别名</h3>
          <el-tag v-for="a in aliases" :key="a" size="small" style="margin:2px;">{{ a }}</el-tag>
        </div>
        <div class="section">
          <h3>信息</h3>
          <p style="color:#888;font-size:13px;">来源: {{ actress.source || '未知' }}</p>
          <p v-if="actress.local_folder_name" style="color:#888;font-size:12px;">目录: {{ actress.local_folder_name }}</p>
        </div>
        <div class="section" v-if="photos.length">
          <h3>照片 ({{ photos.length }})</h3>
          <div class="photo-grid">
            <img v-for="p in photoSrcs" :key="p" :src="p" class="photo-img" @click="previewPhoto(p)" />
          </div>
        </div>
        <div class="section" v-if="movies.length">
          <h3>作品 ({{ movies.length }})</h3>
          <div class="movie-grid">
            <div v-for="m in movies" :key="m.file_id" class="movie-card" @click="$router.push(`/detail/${m.file_id}`)">
              <div class="poster-container">
                <img v-if="m.poster_local" :src="m.poster_b64" class="poster-img" />
                <div v-else class="poster-placeholder"><el-icon :size="28"><VideoCamera /></el-icon></div>
              </div>
              <p class="movie-title">{{ m.title }}</p>
              <p class="movie-year" v-if="m.year">{{ m.year }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
    <!-- Photo preview -->
    <el-dialog v-model="previewVisible" title="照片预览" width="80%">
      <img :src="previewSrc" style="width:100%;max-height:70vh;object-fit:contain;" />
    </el-dialog>
  </div>
  <div v-else class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { Loading, UserFilled, VideoCamera } from '@element-plus/icons-vue'
import type { ActressItem } from '@/types'

const route = useRoute()
const actress = ref<ActressItem | null>(null)
const aliases = ref<string[]>([])
const img = ref('')
const photos = ref<string[]>([])
const photoSrcs = ref<string[]>([])
const movies = ref<any[]>([])
const previewVisible = ref(false)
const previewSrc = ref('')

function previewPhoto(src: string) { previewSrc.value = src; previewVisible.value = true }

onMounted(async () => {
  const id = Number(route.params.id)
  try {
    const result: any = await invoke('get_actresses_paginated', { page: 1, pageSize: 1 })
    const total = result.total || 0
    if (total > 0) {
      const all: any = await invoke('get_actresses_paginated', { page: 1, pageSize: Math.max(total, 1) })
      const found = (all.list || []).find((a: any) => a.id === id)
      if (found) {
        actress.value = found
        aliases.value = await invoke('get_actress_aliases', { actressId: id })
        if (found.avatar_local) {
          try { img.value = await invoke('read_image_base64', { path: found.avatar_local.replace(/\\/g, '/') }) } catch { /* */ }
        }
        // Load photos from folder
        if (found.local_folder_name) {
          photos.value = await invoke('list_folder_images', { dirPath: found.local_folder_name })
          // Load first 50 as base64
          for (const p of photos.value.slice(0, 50)) {
            try { photoSrcs.value.push(await invoke('read_image_base64', { path: p.replace(/\\/g, '/') })) } catch { /* */ }
          }
        }
        // Load movies
        if (found.name) {
          const m: any = await invoke('get_actress_movies', { actressName: found.name })
          movies.value = (m || []).map((x: any) => ({ ...x, poster_b64: '' }))
          for (const mv of movies.value) {
            if (mv.poster_local) {
              try { mv.poster_b64 = await invoke('read_image_base64', { path: mv.poster_local.replace(/\\/g, '/') }) } catch { /* */ }
            }
          }
        }
      }
    }
  } catch(e) { console.error(e) }
})
</script>

<style scoped>
.actress-detail { max-width: 800px; }
.detail-header { margin-bottom: 16px; }
.detail-content { display: flex; gap: 24px; }
.avatar-col { flex-shrink: 0; text-align: center; }
.avatar-large { width: 160px; height: 160px; border-radius: 50%; object-fit: cover; border: 3px solid #3a3a5a; }
.avatar-placeholder { width: 160px; height: 160px; border-radius: 50%; background: #2a2a4a; display: flex; align-items: center; justify-content: center; color: #555; margin: 0 auto; }
.avatar-col h2 { font-size: 20px; margin: 12px 0 8px; }
.meta { display: flex; gap: 6px; flex-wrap: wrap; justify-content: center; }
.info-col { flex: 1; }
.section { margin-bottom: 16px; }
.section h3 { font-size: 15px; margin-bottom: 8px; color: #e0e0e0; }
.loading { text-align: center; padding: 80px; color: #888; }
.photo-grid { display: flex; gap: 8px; flex-wrap: wrap; }
.photo-img { width: 120px; height: 160px; object-fit: cover; border-radius: 4px; cursor: pointer; }
.photo-img:hover { opacity: 0.8; }
.movie-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); gap: 8px; }
.movie-card { cursor: pointer; border-radius: 4px; overflow: hidden; background: #1a1a2e; }
.poster-container { aspect-ratio: 2/3; background: #252540; display: flex; align-items: center; justify-content: center; }
.poster-img { width: 100%; height: 100%; object-fit: cover; }
.poster-placeholder { color: #555; }
.movie-title { font-size: 11px; padding: 4px 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: #c0c0d0; }
.movie-year { font-size: 10px; padding: 0 6px 4px; color: #888; }
</style>
