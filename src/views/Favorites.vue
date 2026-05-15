<template>
  <div class="favorites-page">
    <div class="toolbar">
      <h2>收藏影片</h2>
      <el-input v-model="search" placeholder="搜索..." clearable style="width: 200px" size="small" @change="fetchData" />
      <el-button size="small" @click="groupDialog = true; fetchGroups()">分组管理</el-button>
    </div>

    <div v-if="loading" style="text-align: center; padding: 40px; color: #888;">加载中...</div>

    <div v-else-if="!movies.length" style="text-align: center; padding: 80px; color: #666;">
      <el-icon :size="48"><StarFilled /></el-icon>
      <p style="margin-top: 12px;">暂无收藏影片，在海报墙右键影片可添加到收藏</p>
    </div>

    <div v-else class="movie-grid">
      <div v-for="m in movies" :key="m.file_id" class="movie-card" @click="$router.push(`/detail/${m.file_id}`)" @contextmenu.prevent="onContextMenu($event, m)">
        <div class="poster-container">
          <img v-if="m.poster_local" :src="assetUrl(m.poster_local)" class="poster-img" />
          <div v-else class="poster-placeholder"><el-icon :size="36"><PictureFilled /></el-icon></div>
        </div>
        <div class="movie-info">
          <p class="movie-title">{{ m.title }}</p>
          <p class="movie-meta">
            <span v-if="m.year">{{ m.year }}</span>
            <span v-if="m.rating">★ {{ m.rating.toFixed(1) }}</span>
          </p>
        </div>
      </div>
    </div>

    <!-- 右键菜单 -->
    <div v-if="contextMenu.visible" class="context-menu" :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }">
      <div class="ctx-item" @click="removeFromFavorites">取消收藏</div>
      <div class="ctx-item" @click="addToGroup">添加到分组</div>
    </div>

    <!-- 分组管理对话框 -->
    <el-dialog v-model="groupDialog" title="分组管理" width="420px">
      <div style="margin-bottom: 12px; display: flex; gap: 8px;">
        <el-input v-model="newGroupName" placeholder="新分组名称" size="small" style="flex: 1;" />
        <el-button size="small" type="primary" @click="createGroup">新增</el-button>
      </div>
      <el-table :data="groups" size="small" max-height="300">
        <el-table-column prop="name" label="分组名称" />
        <el-table-column label="操作" width="160">
          <template #default="{ row }">
            <el-button size="small" @click="renameGroup(row)">重命名</el-button>
            <el-button size="small" type="danger" @click="deleteGroup(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import { StarFilled, PictureFilled } from '@element-plus/icons-vue'
import type { MovieItem, GroupItem } from '@/types'

const movies = ref<MovieItem[]>([])
const loading = ref(false)
const search = ref('')
const groups = ref<GroupItem[]>([])
const groupDialog = ref(false)
const newGroupName = ref('')

const contextMenu = reactive({ visible: false, x: 0, y: 0, movie: null as MovieItem | null })

function assetUrl(p: string) { return convertFileSrc(p) }

async function fetchData() {
  loading.value = true
  try {
    const result: any = await invoke('get_movies', { filters: { is_hidden: false }, sort: 'updated_at_desc', page: 1 })
    movies.value = (result.movies || []).filter((m: any) => m.is_favorite)
  } finally { loading.value = false }
}

function onContextMenu(e: MouseEvent, movie: MovieItem) {
  contextMenu.visible = true; contextMenu.x = e.clientX; contextMenu.y = e.clientY; contextMenu.movie = movie
  setTimeout(() => { contextMenu.visible = false }, 3000)
}

async function removeFromFavorites() {
  if (contextMenu.movie) {
    await invoke('set_config', { key: `fav_${contextMenu.movie.file_id}`, value: '0' })
    ElMessage.success('已取消收藏')
    fetchData()
  }
  contextMenu.visible = false
}

async function addToGroup() {
  contextMenu.visible = false
  // Open group selection
}

async function fetchGroups() { groups.value = await invoke('get_groups') }

async function createGroup() {
  if (!newGroupName.value.trim()) return
  await invoke('create_group', { name: newGroupName.value.trim() })
  newGroupName.value = ''
  await fetchGroups()
  ElMessage.success('分组已创建')
}

async function renameGroup(row: GroupItem) {
  const { value } = await ElMessageBox.prompt('新名称', '重命名分组', { inputValue: row.name })
  if (value) { await invoke('rename_group', { groupId: row.id, newName: value }); await fetchGroups() }
}

async function deleteGroup(row: GroupItem) {
  await ElMessageBox.confirm(`确定删除分组「${row.name}」？`, '确认', { type: 'warning' })
  await invoke('delete_group', { groupId: row.id })
  await fetchGroups()
}

onMounted(() => { fetchData(); fetchGroups() })
</script>

<style scoped>
.favorites-page { }
.toolbar { display: flex; gap: 12px; align-items: center; margin-bottom: 16px; padding: 10px; background: #1a1a2e; border-radius: 8px; }
.toolbar h2 { flex: 1; font-size: 16px; }
.movie-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 14px; }
.movie-card { cursor: pointer; border-radius: 6px; overflow: hidden; background: #1a1a2e; transition: transform 0.2s; }
.movie-card:hover { transform: scale(1.03); }
.poster-container { aspect-ratio: 2/3; background: #252540; display: flex; align-items: center; justify-content: center; }
.poster-img { width: 100%; height: 100%; object-fit: cover; }
.poster-placeholder { color: #555; }
.movie-info { padding: 6px; }
.movie-title { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.movie-meta { font-size: 10px; color: #888; margin-top: 2px; }
.context-menu { position: fixed; z-index: 9999; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 120px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
.ctx-item { padding: 8px 16px; cursor: pointer; font-size: 13px; color: #e0e0e0; }
.ctx-item:hover { background: #3a3a5a; }
</style>
