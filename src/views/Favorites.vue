<template>
  <div class="favorites-page">
    <div class="toolbar">
      <h2>收藏影片</h2>
      <el-input v-model="search" placeholder="搜索..." clearable style="width: 180px" size="small" @change="doSearch" />
      <el-button size="small" @click="loadGroups">分组管理</el-button>
    </div>

    <div v-if="loading" class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>

    <div v-else class="movie-grid-wrapper">
      <div v-if="!movies.length" style="text-align: center; padding: 80px; color: #666;">
        <el-icon :size="48"><StarFilled /></el-icon>
        <p style="margin-top: 12px;">暂无收藏影片</p>
      </div>
      <div v-else class="movie-grid">
        <div v-for="m in movies" :key="m.file_id" class="movie-card"
          @click="$router.push(`/detail/${m.file_id}`)"
          @contextmenu.prevent="onContextMenu($event, m)">
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
    </div>

    <div class="table-footer">
      <el-pagination v-if="total > pageSize" :current-page="page"
        :page-size="pageSize" :total="total" layout="prev, pager, next" @current-change="onPageChange" background size="small" />
      <el-select v-model="pageSize" size="small" style="width: 100px; margin-left: 12px;" @change="doSearch">
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
      </el-select>
    </div>

    <!-- 右键菜单 -->
    <div v-if="ctx.visible" class="context-menu" :style="{ left: ctx.x + 'px', top: ctx.y + 'px' }" @mouseleave="ctx.visible = false">
      <div class="ctx-item" @click="removeFavorite">取消收藏</div>
      <div class="ctx-submenu" @mouseenter="ctxSub = 'group'" @mouseleave="ctxSub = ''">
        <div class="ctx-item">📁 添加到分组 ▸</div>
        <div v-if="ctxSub === 'group'" class="sub-menu">
          <div v-for="g in favGroups" :key="'fg_'+g.id" class="ctx-item" @click="addToGroup(g.id)">{{ g.name }}</div>
          <div v-if="!favGroups.length" class="ctx-item" style="color:#666;">暂无分组</div>
        </div>
      </div>
    </div>

    <!-- 分组管理对话框 -->
    <el-dialog v-model="groupDialog" title="收藏分组管理" width="420px">
      <div style="margin-bottom: 12px; display: flex; gap: 8px;">
        <el-input v-model="newGroupName" placeholder="新分组名称" size="small" style="flex: 1;" />
        <el-button size="small" type="primary" @click="createGroup">新增</el-button>
      </div>
      <el-table :data="favGroups" size="small" max-height="300">
        <el-table-column prop="name" label="分组名称" />
        <el-table-column label="操作" width="140">
          <template #default="{ row }">
            <el-button size="small" @click="renameGroup(row)">重命名</el-button>
            <el-button size="small" type="danger" @click="delGroup(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Loading, StarFilled, PictureFilled } from '@element-plus/icons-vue'
import type { MovieItem, GroupItem } from '@/types'

const route = useRoute()
const movies = ref<MovieItem[]>([])
const loading = ref(false)
const search = ref('')
const page = ref(1)
const pageSize = ref(20)
const total = ref(0)
const filterGroupId = ref<number | undefined>()
const favGroups = ref<GroupItem[]>([])

// Context menu
const ctx = reactive({ visible: false, x: 0, y: 0, movie: null as MovieItem | null })
const ctxSub = ref('')

// Group dialog
const groupDialog = ref(false)
const newGroupName = ref('')

function assetUrl(p: string) { return convertFileSrc(p) }

watch(() => route.query.group_id, (val) => {
  filterGroupId.value = val ? Number(val) : undefined; doSearch()
}, { immediate: true })

async function doSearch() { page.value = 1; fetchData() }

async function fetchData() {
  loading.value = true
  try {
    const result: any = await invoke('get_movies', {
      filters: { group_id: filterGroupId.value, is_hidden: false },
      sort: 'updated_at_desc', page: page.value, pageSize: pageSize.value,
    })
    movies.value = result.movies || []
    total.value = result.total || 0
  } finally { loading.value = false }
}

function onPageChange(p: number) { page.value = p; fetchData() }

async function loadGroups() {
  favGroups.value = await invoke('get_groups', { category: 'favorite' })
  groupDialog.value = true
}

function onContextMenu(e: MouseEvent, movie: MovieItem) {
  ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.movie = movie
  invoke('get_groups', { category: 'favorite' }).then((g: any) => favGroups.value = g || [])
}

async function removeFavorite() {
  if (!ctx.movie) return; ctx.visible = false
  ElMessage.info('取消收藏功能待实现')
}

async function addToGroup(groupId: number) {
  if (!ctx.movie) return
  await invoke('add_movies_to_group', { groupId, fileIds: [ctx.movie.file_id] })
  ElMessage.success('已添加到分组'); ctx.visible = false
}

async function createGroup() {
  if (!newGroupName.value.trim()) return
  await invoke('create_group', { name: newGroupName.value.trim(), groupType: 'favorite' })
  newGroupName.value = ''
  favGroups.value = await invoke('get_groups', { category: 'favorite' })
  ElMessage.success('分组已创建')
}

async function renameGroup(row: GroupItem) {
  const { value } = await ElMessageBox.prompt('新名称', '重命名', { inputValue: row.name })
  if (value) { await invoke('rename_group', { groupId: row.id, newName: value }); favGroups.value = await invoke('get_groups', { category: 'favorite' }) }
}

async function delGroup(row: GroupItem) {
  await ElMessageBox.confirm(`确定删除「${row.name}」？`, '确认', { type: 'warning' })
  await invoke('delete_group', { groupId: row.id })
  favGroups.value = await invoke('get_groups', { category: 'favorite' })
}

onMounted(() => { filterGroupId.value = route.query.group_id ? Number(route.query.group_id) : undefined; fetchData() })
</script>

<style scoped>
.favorites-page { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.toolbar { flex-shrink: 0; display: flex; gap: 10px; align-items: center; margin-bottom: 14px; padding: 10px 14px; background: #1a1a2e; border-radius: 8px; }
.toolbar h2 { flex: 1; font-size: 16px; }
.loading { text-align: center; padding: 60px; color: #888; }
.movie-grid-wrapper { flex: 1; overflow-y: auto; }
.movie-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 14px; }
.movie-card { cursor: pointer; border-radius: 6px; overflow: hidden; background: #1a1a2e; transition: transform 0.2s; }
.movie-card:hover { transform: scale(1.03); }
.poster-container { aspect-ratio: 2/3; background: #252540; display: flex; align-items: center; justify-content: center; }
.poster-img { width: 100%; height: 100%; object-fit: cover; }
.poster-placeholder { color: #555; }
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
