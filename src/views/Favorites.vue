<template>
  <div class="favorites-page">
    <div class="toolbar">
      <el-input v-model="search" placeholder="搜索影片..." clearable style="width: 240px" size="small" @clear="doSearch" @keyup.enter="doSearch" />
      <el-select v-model="filterGenre" placeholder="类型筛选" clearable style="width: 160px" size="small" @change="doSearch">
        <el-option v-for="g in allGenres" :key="g" :label="g" :value="g" />
      </el-select>
      <el-select v-model="filterYear" placeholder="年份筛选" clearable style="width: 120px" size="small" @change="doSearch">
        <el-option v-for="y in years" :key="y" :label="String(y)" :value="y" />
      </el-select>
      <el-button type="primary" size="small" @click="doSearch">筛选</el-button>
    </div>

    <div v-if="loading" class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>

    <div v-else class="movie-grid-wrapper">
      <div v-if="!movies.length" style="text-align: center; padding: 80px; color: #666;">
        <el-icon :size="48"><StarFilled /></el-icon>
        <p style="margin-top: 12px;">暂无收藏影片，在海报墙右键影片可添加到收藏分组</p>
      </div>
      <div v-else class="movie-grid">
        <div v-for="m in movies" :key="m.file_id" class="movie-card"
          @click="$router.push(`/detail/${m.file_id}`)"
          @contextmenu.prevent="onContextMenu($event, m)">
          <div class="poster-container">
            <img v-if="m.poster_local" :src="posterUrl(m)" class="poster-img" />
            <div v-else class="poster-placeholder"><el-icon :size="40"><PictureFilled /></el-icon></div>
          </div>
          <div v-if="m.progress" class="progress-bar">
            <el-progress :percentage="Math.round(m.progress / (m.duration || 1) * 100)" :stroke-width="3" />
          </div>
          <div class="movie-info">
            <p class="movie-title" :title="m.title">{{ m.title }}</p>
            <p class="movie-meta">
              <span v-if="m.year">{{ m.year }}</span>
              <span v-if="m.rating">★ {{ m.rating.toFixed(1) }}</span>
            </p>
          </div>
        </div>
      </div>
    </div>

    <div class="table-footer">
      <el-pagination v-if="total > pageSize" :current-page="page"
        :page-size="pageSize" :total="total" layout="prev, pager, next" @current-change="onPageChange" background size="small" />
      <el-select v-model="pageSize" size="small" style="width: 100px; margin-left: 12px;" @change="onPageSizeChange">
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
      </el-select>
    </div>

    <!-- 右键菜单（与海报墙完全一致） -->
    <div v-if="ctx.visible" class="context-menu" :style="{ left: ctx.x + 'px', top: ctx.y + 'px' }" @mouseleave="ctx.visible = false">
      <div class="ctx-submenu" @mouseenter="ctxSub = 'fav'" @mouseleave="ctxSub = ''">
        <div class="ctx-item">⭐ 添加到收藏 ▸</div>
        <div v-if="ctxSub === 'fav'" class="sub-menu">
          <div v-for="g in favGroups" :key="'f_'+g.id" class="ctx-item" @click="addToFavGroup(g.id)">{{ g.name }}</div>
          <div v-if="!favGroups.length" class="ctx-item" style="color:#666;">暂无收藏分组</div>
        </div>
      </div>
      <div class="ctx-submenu" @mouseenter="ctxSub = 'poster'" @mouseleave="ctxSub = ''">
        <div class="ctx-item">📁 添加到分组 ▸</div>
        <div v-if="ctxSub === 'poster'" class="sub-menu">
          <div v-for="g in posterGroups" :key="'p_'+g.id" class="ctx-item" @click="addToPosterGroup(g.id)">{{ g.name }}</div>
          <div v-if="!posterGroups.length" class="ctx-item" style="color:#666;">暂无海报墙分组</div>
        </div>
      </div>
      <div class="ctx-item" @click="rescrapeMovie">🔄 重新刮削</div>
      <div class="ctx-item danger" @click="removeFavorite">取消收藏</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { Loading, StarFilled, PictureFilled } from '@element-plus/icons-vue'
import { imageUrl } from '@/composables/useImageUrl'
import type { MovieItem, GroupItem } from '@/types'

const route = useRoute()
const movies = ref<MovieItem[]>([])
const loading = ref(false)
const search = ref('')
const filterGenre = ref('')
const filterYear = ref<number | undefined>()
const page = ref(1)
const pageSize = ref(20)
const total = ref(0)
const filterGroupId = ref<number | undefined>()

const allGenres = ['动作', '科幻', '喜剧', '爱情', '恐怖', '剧情', '悬疑', '动画', '纪录片']
const years = Array.from({ length: 40 }, (_, i) => new Date().getFullYear() - i)

// Context menu
const ctx = reactive({ visible: false, x: 0, y: 0, movie: null as MovieItem | null })
const ctxSub = ref('')
const favGroups = ref<GroupItem[]>([])
const posterGroups = ref<GroupItem[]>([])

function posterUrl(m: MovieItem): string {
  return imageUrl(m.poster_local)
}

watch(() => route.query.group_id, (val) => {
  filterGroupId.value = val ? Number(val) : undefined; doSearch()
}, { immediate: true })

function doSearch() { page.value = 1; fetchData() }
function onPageChange(p: number) { page.value = p; fetchData() }
function onPageSizeChange() { page.value = 1; fetchData() }

async function fetchData() {
  loading.value = true
  try {
    const result: any = await invoke('get_movies', {
      filters: {
        keyword: search.value || undefined,
        year: filterYear.value,
        genre: filterGenre.value || undefined,
        group_id: filterGroupId.value,
        is_hidden: false,
        favorites_only: true,
      },
      sort: 'updated_at_desc', page: page.value, pageSize: pageSize.value,
    })
    // favorites_only backend filter + frontend fallback
    movies.value = (result.movies || []).filter((m: any) => m.is_favorite !== false)
    total.value = result.total || 0
  } finally { loading.value = false }
}

function onContextMenu(e: MouseEvent, movie: MovieItem) {
  ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.movie = movie
  invoke('get_groups', { category: 'favorite' }).then((g: any) => favGroups.value = g || [])
  invoke('get_groups', { category: 'manual' }).then((g: any) => posterGroups.value = g || [])
}

async function addToFavGroup(groupId: number) {
  if (!ctx.movie) return
  await invoke('add_movies_to_group', { groupId, fileIds: [ctx.movie.file_id] })
  ElMessage.success('已添加到收藏分组'); ctx.visible = false
}

async function addToPosterGroup(groupId: number) {
  if (!ctx.movie) return
  await invoke('add_movies_to_group', { groupId, fileIds: [ctx.movie.file_id] })
  ElMessage.success('已添加到分组'); ctx.visible = false
}

async function rescrapeMovie() {
  if (!ctx.movie) return
  ctx.visible = false
  try {
    const result: any = await invoke('scrape_batch', { fileIds: [ctx.movie.file_id] })
    ElMessage.success(`刮削完成: 成功${result.success}, 失败${result.failed}`)
    fetchData()
  } catch (e: any) {
    ElMessage.error('刮削失败: ' + (e?.message || e))
  }
}

async function removeFavorite() {
  if (!ctx.movie) return
  ctx.visible = false
  try {
    // Find which favorite group this movie belongs to and remove it
    const groups: any[] = await invoke('get_groups', { category: 'favorite' })
    for (const g of groups) {
      await invoke('remove_movie_from_group', { groupId: g.id, fileId: ctx.movie.file_id }).catch(() => {})
    }
    ElMessage.success('已取消收藏')
    fetchData()
  } catch (e: any) {
    ElMessage.error('操作失败: ' + (e?.message || e))
  }
}

function onNavRefresh(e: Event) {
  const ce = e as CustomEvent
  if (ce.detail.path === '/favorites') {
    filterGroupId.value = ce.detail.query.group_id || undefined
    doSearch()
  }
}

onMounted(() => {
  window.addEventListener('nav-refresh', onNavRefresh)
})

onUnmounted(() => {
  window.removeEventListener('nav-refresh', onNavRefresh)
})
</script>

<style scoped>
.favorites-page { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.toolbar { flex-shrink: 0; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-bottom: 14px; padding: 10px 14px; background: #1a1a2e; border-radius: 8px; }
.loading { text-align: center; padding: 60px; color: #888; }
.movie-grid-wrapper { flex: 1; overflow-y: auto; }
.movie-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 14px; }
.movie-card { cursor: pointer; border-radius: 6px; overflow: hidden; background: #1a1a2e; transition: transform 0.2s; }
.movie-card:hover { transform: scale(1.03); }
.poster-container { aspect-ratio: 2/3; background: #252540; display: flex; align-items: center; justify-content: center; position: relative; }
.poster-img { width: 100%; height: 100%; object-fit: cover; }
.poster-placeholder { color: #555; }
.progress-bar { position: absolute; bottom: 0; left: 0; right: 0; }
.movie-info { padding: 6px; }
.movie-title { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.movie-meta { font-size: 10px; color: #888; margin-top: 2px; }
.table-footer { flex-shrink: 0; display: flex; justify-content: center; align-items: center; padding: 10px 0; }
.context-menu { position: fixed; z-index: 9999; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 150px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
.ctx-item { padding: 8px 16px; cursor: pointer; font-size: 13px; color: #c0c0d0; white-space: nowrap; }
.ctx-item:hover { background: #3a3a5a; color: #fff; }
.ctx-submenu { position: relative; }
.sub-menu { position: absolute; left: 100%; top: 0; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 140px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
</style>
