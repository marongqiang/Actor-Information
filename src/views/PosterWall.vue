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

    <div v-else class="movie-grid">
      <div
        v-for="movie in store.movies"
        :key="movie.file_id"
        class="movie-card"
        @click="$router.push(`/detail/${movie.file_id}`)"
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

    <el-pagination
      v-if="store.total > 0"
      v-model:current-page="currentPage"
      :page-size="20"
      :total="store.total"
      layout="prev, pager, next"
      @current-change="onPageChange"
      style="margin-top: 20px; justify-content: center;"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { convertFileSrc } from '@tauri-apps/api/core'
import { Loading, PictureFilled } from '@element-plus/icons-vue'

const store = useLibraryStore()
const searchKeyword = ref('')
const filterGenre = ref('')
const filterYear = ref<number | undefined>(undefined)
const filterGroup = ref<number | undefined>(undefined)
const showHidden = ref(false)
const currentPage = ref(1)

const allGenres = ['动作', '科幻', '喜剧', '爱情', '恐怖', '剧情', '悬疑', '动画', '纪录片']
const years = computed(() => {
  const y = new Date().getFullYear()
  return Array.from({ length: 40 }, (_, i) => y - i)
})

function assetUrl(path: string) {
  return convertFileSrc(path)
}

function doSearch() {
  store.setFilters({
    keyword: searchKeyword.value || undefined,
    genre: filterGenre.value || undefined,
    year: filterYear.value,
    group_id: filterGroup.value,
    is_hidden: showHidden.value || undefined,
  })
}

function onPageChange(page: number) {
  currentPage.value = page
  store.fetchMovies(page)
}

onMounted(async () => {
  await store.fetchGroups()
  await store.fetchMovies(1)
})
</script>

<style scoped>
.poster-wall { }
.toolbar {
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
.movie-meta { font-size: 11px; color: #888; margin-top: 4px; display: flex; gap: 8px; }
</style>
