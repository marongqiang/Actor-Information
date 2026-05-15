<template>
  <div class="actress-page">
    <div class="toolbar">
      <h2>演员库</h2>
      <el-input v-model="search" placeholder="搜索演员..." clearable style="width: 240px" @change="doSearch" />
      <el-button type="primary" @click="store.syncData()">同步数据</el-button>
    </div>

    <div v-if="store.loading" class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>

    <div v-else class="actress-grid">
      <div v-for="actress in store.actresses" :key="actress.id" class="actress-card">
        <div class="avatar-container">
          <img v-if="actress.avatar_local" :src="assetUrl(actress.avatar_local)" class="avatar-img" />
          <div v-else class="avatar-placeholder">
            <el-icon :size="36"><UserFilled /></el-icon>
          </div>
        </div>
        <div class="actress-info">
          <p class="actress-name">{{ actress.name }}</p>
          <p class="actress-meta">
            <span v-if="actress.debut_year">{{ actress.debut_year }}年出道</span>
            <span v-if="actress.cup">{{ actress.cup }}</span>
            <span v-if="actress.movie_count">作品 {{ actress.movie_count }}</span>
          </p>
        </div>
      </div>
    </div>

    <el-pagination
      v-if="store.total > 20"
      v-model:current-page="page"
      :page-size="20"
      :total="store.total"
      layout="prev, pager, next"
      @current-change="onPageChange"
      style="margin-top: 20px; justify-content: center;"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useActressStore } from '@/stores/actress'
import { convertFileSrc } from '@tauri-apps/api/core'
import { Loading, UserFilled } from '@element-plus/icons-vue'

const store = useActressStore()
const search = ref('')
const page = ref(1)

function assetUrl(path: string) {
  return convertFileSrc(path)
}

function doSearch() {
  page.value = 1
  store.fetchPaginated(1, 20, search.value || undefined)
}

function onPageChange(p: number) {
  page.value = p
  store.fetchPaginated(p, 20, search.value || undefined)
}

onMounted(() => {
  store.fetchPaginated(1, 20)
})
</script>

<style scoped>
.actress-page { }
.toolbar {
  display: flex; gap: 12px; align-items: center;
  margin-bottom: 20px; padding: 12px; background: #1a1a2e; border-radius: 8px;
}
.toolbar h2 { flex: 1; font-size: 18px; }
.loading { text-align: center; padding: 60px; color: #888; }
.actress-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 12px;
}
.actress-card {
  background: #1a1a2e; border-radius: 8px; overflow: hidden;
  cursor: pointer; transition: transform 0.2s;
}
.actress-card:hover { transform: scale(1.03); }
.avatar-container { aspect-ratio: 3/4; overflow: hidden; background: #2a2a4a; }
.avatar-img { width: 100%; height: 100%; object-fit: cover; }
.avatar-placeholder {
  display: flex; align-items: center; justify-content: center;
  height: 100%; color: #555;
}
.actress-info { padding: 8px; }
.actress-name { font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.actress-meta { font-size: 11px; color: #888; margin-top: 4px; display: flex; gap: 6px; flex-wrap: wrap; }
</style>
