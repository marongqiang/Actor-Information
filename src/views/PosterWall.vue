<template>
  <div class="poster-wall">
    <div class="toolbar">
      <el-input
        v-model="searchKeyword"
        placeholder="搜索影片..."
        clearable
        style="width: 240px"
        @clear="doSearch"
        @keyup.enter="doSearch"
      />
      <el-select v-model="filterGenre" placeholder="类型筛选" clearable style="width: 160px" @change="doSearch">
        <el-option v-for="g in allGenres" :key="g" :label="g" :value="g" />
      </el-select>
      <el-select v-model="filterYear" placeholder="年份筛选" clearable style="width: 120px" @change="doSearch">
        <el-option v-for="y in years" :key="y" :label="String(y)" :value="y" />
      </el-select>
      <el-select v-model="filterGroup" placeholder="分组筛选" clearable style="width: 140px" @change="doSearch">
        <el-option v-for="g in store.groups" :key="g.id" :label="g.name" :value="g.id" />
      </el-select>
      <el-switch v-model="showHidden" active-text="已隐藏" @change="doSearch" />
      <el-button type="primary" @click="doSearch">筛选</el-button>
    </div>

    <div v-if="store.loading" class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>

    <div v-else class="movie-grid-wrapper">
    <div class="movie-grid">
      <div
        v-for="movie in store.movies"
        :key="movie.file_id"
        class="movie-card"
        @click="$router.push(`/detail/${movie.file_id}`)"
        @contextmenu.prevent="onContextMenu($event, movie)"
      >
        <div class="poster-container">
          <img
            v-if="movie.poster_local"
            :src="assetUrl(movie.poster_local)"
            :alt="movie.title"
            class="poster-img"
          />
          <div v-else class="poster-placeholder">
            <el-icon :size="48"><PictureFilled /></el-icon>
          </div>
          <div v-if="movie.progress" class="progress-bar">
            <el-progress :percentage="Math.round(movie.progress / (movie.duration || 1) * 100)" :stroke-width="3" />
          </div>
        </div>
        <div class="movie-info">
          <p class="movie-title" :title="movie.title">{{ movie.title }}</p>
          <p class="movie-meta">
            <span v-if="movie.year">{{ movie.year }}</span>
            <span v-if="movie.rating">★ {{ movie.rating.toFixed(1) }}</span>
          </p>
        </div>
      </div>
    </div>
    </div>

    <div class="table-footer">
      <el-pagination v-if="store.total > pageSize" :current-page="currentPage"
        :page-size="pageSize" :total="store.total" layout="prev, pager, next" @current-change="onPageChange" background size="small" />
      <el-select v-model="pageSize" size="small" style="width: 100px; margin-left: 12px;" @change="onPageSizeChange">
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
      </el-select>
    </div>

    <!-- 右键菜单 -->
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
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, reactive, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useLibraryStore } from '@/stores/library'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { Loading, PictureFilled } from '@element-plus/icons-vue'
import type { MovieItem, GroupItem } from '@/types'

const route = useRoute()
const store = useLibraryStore()
const searchKeyword = ref('')
const filterGenre = ref('')
const filterYear = ref<number | undefined>(undefined)
const filterGroup = ref<number | undefined>(undefined)
const showHidden = ref(false)
const currentPage = ref(1)
const pageSize = ref(20)

watch(() => route.query.group_id, (val) => {
  filterGroup.value = val ? Number(val) : undefined
  doSearch()
}, { immediate: true })

// Context menu
const ctx = reactive({ visible: false, x: 0, y: 0, movie: null as MovieItem | null })
const ctxSub = ref('')
const favGroups = ref<GroupItem[]>([])
const posterGroups = ref<GroupItem[]>([])

const allGenres = ['动作', '科幻', '喜剧', '爱情', '恐怖', '剧情', '悬疑', '动画', '纪录片']
const years = computed(() => {
  const y = new Date().getFullYear()
  return Array.from({ length: 40 }, (_, i) => y - i)
})

function assetUrl(path: string) {
  return convertFileSrc(path)
}

function doSearch() {
  currentPage.value = 1
  store.setFilters({
    keyword: searchKeyword.value || undefined,
    genre: filterGenre.value || undefined,
    year: filterYear.value,
    group_id: filterGroup.value,
    is_hidden: showHidden.value || undefined,
  })
}

function onPageChange(p: number) { currentPage.value = p; store.fetchMovies(p, pageSize.value) }
function onPageSizeChange() { currentPage.value = 1; store.fetchMovies(1, pageSize.value) }

// Context menu handlers
function onContextMenu(e: MouseEvent, movie: MovieItem) {
  ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.movie = movie
  // Load groups
  invoke('get_groups', { category: 'favorite' }).then((g: any) => favGroups.value = g || [])
  invoke('get_groups', { category: 'manual' }).then((g: any) => posterGroups.value = g || [])
}

async function addToFavGroup(groupId: number) {
  if (!ctx.movie) return
  await invoke('add_movies_to_group', { groupId, fileIds: [ctx.movie.file_id] })
  ElMessage.success('已添加到收藏分组')
  ctx.visible = false
}

async function addToPosterGroup(groupId: number) {
  if (!ctx.movie) return
  await invoke('add_movies_to_group', { groupId, fileIds: [ctx.movie.file_id] })
  ElMessage.success('已添加到分组')
  ctx.visible = false
}

async function rescrapeMovie() {
  if (ctx.movie) ElMessage.info('重新刮削功能待实现')
  ctx.visible = false
}

onMounted(async () => {
  await store.fetchGroups()
  doSearch()
})
</script>

<style scoped>
.poster-wall { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.movie-grid-wrapper { flex: 1; overflow-y: auto; }
.toolbar { flex-shrink: 0;
  display: flex; gap: 12px; flex-wrap: wrap; align-items: center;
  margin-bottom: 20px; padding: 12px; background: #1a1a2e; border-radius: 8px;
}
.loading { text-align: center; padding: 60px; color: #888; }
.movie-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 16px;
}
.movie-card {
  cursor: pointer; border-radius: 8px; overflow: hidden;
  background: #1a1a2e; transition: transform 0.2s;
}
.movie-card:hover { transform: scale(1.03); }
.poster-container { position: relative; aspect-ratio: 2/3; overflow: hidden; background: #2a2a4a; }
.poster-img { width: 100%; height: 100%; object-fit: cover; }
.poster-placeholder {
  display: flex; align-items: center; justify-content: center;
  height: 100%; color: #555;
}
.progress-bar { position: absolute; bottom: 0; left: 0; right: 0; }
.movie-info { padding: 8px; }
.movie-title { font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.table-footer { flex-shrink: 0; display: flex; justify-content: center; align-items: center; padding: 12px 0; }
.movie-meta { font-size: 11px; color: #888; margin-top: 4px; display: flex; gap: 8px; }
.context-menu { position: fixed; z-index: 9999; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 150px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
.ctx-item { padding: 8px 16px; cursor: pointer; font-size: 13px; color: #c0c0d0; white-space: nowrap; }
.ctx-item:hover { background: #3a3a5a; color: #fff; }
.ctx-submenu { position: relative; }
.sub-menu { position: absolute; left: 100%; top: 0; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 140px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
</style>
