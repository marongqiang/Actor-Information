<template>
  <div class="actress-page">
    <div class="toolbar">
      <h2>演员库</h2>
      <el-input v-model="search" placeholder="搜索演员..." clearable style="width: 200px" size="small" @change="doSearch" />
      <el-button type="primary" size="small" @click="store.syncData()">同步数据</el-button>
      <el-button size="small" @click="showGroupManage">分组管理</el-button>
    </div>

    <div v-if="store.loading" class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>

    <div v-else class="actress-grid">
      <div v-for="actress in store.actresses" :key="actress.id" class="actress-card"
          @contextmenu.prevent="onContextMenu($event, actress)">
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

    <!-- 右键菜单 -->
    <div v-if="ctx.visible" class="context-menu" :style="{ left: ctx.x + 'px', top: ctx.y + 'px' }" @mouseleave="ctx.visible = false">
      <div class="ctx-item" @click="addToGroup">📁 添加到演员分组</div>
      <div class="ctx-item" @click="viewMovies">🎬 查看关联影片</div>
    </div>

    <!-- 分组管理对话框 -->
    <el-dialog v-model="groupDialog" title="演员分组管理" width="420px">
      <div style="margin-bottom: 12px; display: flex; gap: 8px;">
        <el-input v-model="newGroupName" placeholder="新分组名称" size="small" style="flex: 1;" />
        <el-button size="small" type="primary" @click="createGroup">新增</el-button>
      </div>
      <el-table :data="actressGroups" size="small" max-height="300">
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
import { ref, onMounted, reactive } from 'vue'
import { useActressStore } from '@/stores/actress'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Loading, UserFilled } from '@element-plus/icons-vue'
import type { ActressItem, ActressGroupItem } from '@/types'

const store = useActressStore()
const search = ref('')
const page = ref(1)

// Context menu
const ctx = reactive({ visible: false, x: 0, y: 0, actress: null as ActressItem | null })

// Group management
const groupDialog = ref(false)
const newGroupName = ref('')
const actressGroups = ref<ActressGroupItem[]>([])

function assetUrl(path: string) { return convertFileSrc(path) }

function doSearch() { page.value = 1; store.fetchPaginated(1, 20, search.value || undefined) }
function onPageChange(p: number) { page.value = p; store.fetchPaginated(p, 20, search.value || undefined) }

function onContextMenu(e: MouseEvent, actress: ActressItem) {
  ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.actress = actress
}

async function addToGroup() {
  if (!ctx.actress) return
  ctx.visible = false
  // Simple implementation: use existing actress group system
  const groups = await invoke('get_actress_groups')
  actressGroups.value = groups as ActressGroupItem[]
  groupDialog.value = true
}

function viewMovies() {
  if (ctx.actress) ElMessage.info(`查看 ${ctx.actress.name} 的关联影片（待实现）`)
  ctx.visible = false
}

async function showGroupManage() {
  actressGroups.value = await invoke('get_actress_groups')
  groupDialog.value = true
}

async function createGroup() {
  if (!newGroupName.value.trim()) return
  await invoke('create_actress_group', { name: newGroupName.value.trim() })
  newGroupName.value = ''
  actressGroups.value = await invoke('get_actress_groups')
  ElMessage.success('分组已创建')
}

async function renameGroup(row: ActressGroupItem) {
  const { value } = await ElMessageBox.prompt('新名称', '重命名', { inputValue: row.name })
  if (value) {
    await invoke('rename_actress_group', { groupId: row.id, newName: value })
    actressGroups.value = await invoke('get_actress_groups')
  }
}

async function deleteGroup(row: ActressGroupItem) {
  await ElMessageBox.confirm(`确定删除分组「${row.name}」？`, '确认', { type: 'warning' })
  await invoke('delete_actress_group', { groupId: row.id })
  actressGroups.value = await invoke('get_actress_groups')
}

onMounted(() => { store.fetchPaginated(1, 20) })
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
.context-menu { position: fixed; z-index: 9999; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 140px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
.ctx-item { padding: 8px 16px; cursor: pointer; font-size: 13px; color: #c0c0d0; }
.ctx-item:hover { background: #3a3a5a; color: #fff; }
</style>
