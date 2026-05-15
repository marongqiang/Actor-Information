<template>
  <div id="app-container">
    <el-container>
      <el-aside width="200px" class="sidebar">
        <div class="logo" @click="$router.push('/')">
          <h2>智能网盘影视库</h2>
        </div>

        <!-- 自定义导航（支持右键分组管理和子项展开） -->
        <nav class="nav-list">
          <!-- 海报墙 + 影片分组 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/' }"
              @click="$router.push('/')"
              @contextmenu.prevent="openGroupMenu($event, 'movie')">
              <el-icon><PictureFilled /></el-icon>
              <span>海报墙</span>
            </div>
            <div v-for="g in movieGroups" :key="'mg_'+g.id" class="nav-sub-item"
              :class="{ active: currentRoute.startsWith('/group/') && currentRoute.endsWith('/'+g.id) }"
              @click="$router.push(`/?group_id=${g.id}`)">
              {{ g.name }}
              <span class="badge">{{ g.movie_count || 0 }}</span>
            </div>
          </div>

          <!-- 收藏影片 + 分组 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/favorites' }"
              @click="$router.push('/favorites')"
              @contextmenu.prevent="openGroupMenu($event, 'movie')">
              <el-icon><StarFilled /></el-icon>
              <span>收藏影片</span>
            </div>
          </div>

          <!-- 演员库 + 演员分组 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/actress' || currentRoute === '/actress-table' }"
              @click="toggleActressSub"
              @contextmenu.prevent="openGroupMenu($event, 'actress')">
              <el-icon><UserFilled /></el-icon>
              <span>演员库</span>
              <el-icon class="arrow" :class="{ open: actressExpanded }"><ArrowRight /></el-icon>
            </div>
            <div v-if="actressExpanded">
              <div class="nav-sub-item" :class="{ active: currentRoute === '/actress' }"
                @click="$router.push('/actress')">演员墙</div>
              <div class="nav-sub-item" :class="{ active: currentRoute === '/actress-table' }"
                @click="$router.push('/actress-table')">演员表格</div>
              <div v-for="g in actressGroups" :key="'ag_'+g.id" class="nav-sub-item"
                @click="$router.push(`/actress-table?group_id=${g.id}`)">
                {{ g.name }}
                <span class="badge">{{ g.member_count || 0 }}</span>
              </div>
            </div>
          </div>

          <!-- 扫描管理 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/scan' }"
              @click="$router.push('/scan')">
              <el-icon><FolderOpened /></el-icon>
              <span>扫描管理</span>
            </div>
          </div>

          <!-- 设置 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/settings' }"
              @click="$router.push('/settings')">
              <el-icon><Setting /></el-icon>
              <span>设置</span>
            </div>
          </div>
        </nav>

        <div class="sidebar-footer">
          <el-progress v-if="scanTask" :percentage="scanTask.progress"
            :status="scanTask.status === 'failed' ? 'exception' : undefined" :stroke-width="6" />
          <p v-if="scanTask" class="task-hint">扫描: {{ scanTask.progress }}%</p>
        </div>
      </el-aside>
      <el-main>
        <router-view />
      </el-main>
    </el-container>

    <!-- 右键菜单 -->
    <div v-if="ctx.visible" class="context-menu" :style="{ left: ctx.x + 'px', top: ctx.y + 'px' }" @mouseleave="ctx.visible = false">
      <div class="ctx-item" @click="showAddGroup">➕ 新增分组</div>
      <div class="ctx-item" @click="showRenameGroup">✏️ 重命名分组</div>
      <div class="ctx-item danger" @click="showDeleteGroup">🗑️ 删除分组</div>
    </div>

    <!-- 新增/重命名分组对话框 -->
    <el-dialog v-model="groupFormDialog" :title="groupFormTitle" width="380px">
      <el-input v-model="groupFormName" placeholder="分组名称" style="margin-bottom: 12px;" />
      <template #footer>
        <el-button @click="groupFormDialog = false">取消</el-button>
        <el-button type="primary" @click="confirmGroupForm">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, reactive } from 'vue'
import { useRoute } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  PictureFilled, UserFilled, Grid, FolderOpened, Setting, StarFilled, ArrowRight
} from '@element-plus/icons-vue'
import type { Task, GroupItem, ActressGroupItem } from '@/types'

const route = useRoute()
const currentRoute = computed(() => route.path)
const scanTask = ref<Task | null>(null)

const actressExpanded = ref(true)
const movieGroups = ref<GroupItem[]>([])
const actressGroups = ref<ActressGroupItem[]>([])

// Context menu
const ctx = reactive({ visible: false, x: 0, y: 0, type: 'movie' as 'movie' | 'actress' })

// Group form
const groupFormDialog = ref(false)
const groupFormTitle = ref('')
const groupFormName = ref('')
let groupFormMode: 'add' | 'rename' | 'delete' = 'add'
let editingGroupId: number | null = null

function toggleActressSub() {
  actressExpanded.value = !actressExpanded.value
}

// ─── 右键分组菜单 ───

async function openGroupMenu(e: MouseEvent, type: 'movie' | 'actress') {
  e.preventDefault()
  ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.type = type
}

function showAddGroup() {
  ctx.visible = false
  groupFormMode = 'add'; editingGroupId = null
  groupFormTitle.value = `新建${ctx.type === 'movie' ? '影片' : '演员'}分组`
  groupFormName.value = ''
  groupFormDialog.value = true
}

async function showRenameGroup() {
  ctx.visible = false
  // 让用户选择要重命名的分组
  const groups = ctx.type === 'movie' ? movieGroups.value : actressGroups.value
  if (!groups.length) { ElMessage.warning('暂无分组'); return }

  const names = groups.map(g => g.name)
  // 简单实现：弹出输入框
  const { value: sel } = await ElMessageBox.prompt('输入要重命名的分组名称（精确匹配）', '选择分组', { inputType: 'text' })
  if (!sel) return

  const found = groups.find(g => g.name === sel)
  if (!found) { ElMessage.warning('未找到该分组'); return }

  groupFormMode = 'rename'; editingGroupId = found.id
  groupFormTitle.value = `重命名分组「${found.name}」`
  groupFormName.value = found.name
  groupFormDialog.value = true
}

async function showDeleteGroup() {
  ctx.visible = false
  const groups = ctx.type === 'movie' ? movieGroups.value : actressGroups.value
  if (!groups.length) { ElMessage.warning('暂无分组'); return }

  const { value: sel } = await ElMessageBox.prompt('输入要删除的分组名称（精确匹配）', '删除分组', { inputType: 'text' })
  if (!sel) return

  const found = groups.find(g => g.name === sel)
  if (!found) { ElMessage.warning('未找到该分组'); return }

  await ElMessageBox.confirm(`确定删除分组「${found.name}」及其关联数据？`, '确认删除', { type: 'warning' })

  if (ctx.type === 'movie') {
    await invoke('delete_group', { groupId: found.id })
  } else {
    await invoke('delete_actress_group', { groupId: found.id })
  }
  ElMessage.success('已删除')
  await fetchGroups()
}

async function confirmGroupForm() {
  if (!groupFormName.value.trim()) { ElMessage.warning('请输入名称'); return }

  if (groupFormMode === 'add') {
    if (ctx.type === 'movie') {
      await invoke('create_group', { name: groupFormName.value.trim() })
    } else {
      await invoke('create_actress_group', { name: groupFormName.value.trim() })
    }
    ElMessage.success('分组已创建')
  } else if (groupFormMode === 'rename' && editingGroupId) {
    if (ctx.type === 'movie') {
      await invoke('rename_group', { groupId: editingGroupId, newName: groupFormName.value.trim() })
    } else {
      await invoke('rename_actress_group', { groupId: editingGroupId, newName: groupFormName.value.trim() })
    }
    ElMessage.success('已重命名')
  }

  groupFormDialog.value = false
  await fetchGroups()
}

async function fetchGroups() {
  try {
    movieGroups.value = await invoke('get_groups')
    actressGroups.value = await invoke('get_actress_groups')
  } catch { /* ignore */ }
}

onMounted(async () => {
  await fetchGroups()
  await listen('scan-progress', (event: any) => {
    scanTask.value = {
      id: '', type: 'scan', status: 'running',
      progress: event.payload.percent || 0, created_at: 0, updated_at: 0,
    }
  })
})
</script>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: 'Microsoft YaHei', sans-serif; background: #0f0f1a; color: #e0e0e0; }
#app-container { height: 100vh; }
.el-container { height: 100%; }

.sidebar {
  background: #1a1a2e; display: flex; flex-direction: column;
  border-right: 1px solid #2a2a4a; user-select: none;
}
.logo { padding: 16px 14px; cursor: pointer; border-bottom: 1px solid #2a2a4a; }
.logo h2 { font-size: 15px; color: #409eff; text-align: center; }

/* 自定义导航 */
.nav-list { flex: 1; overflow-y: auto; padding: 4px 0; }
.nav-section { padding: 2px 0; border-bottom: 1px solid #22223a; }
.nav-section:last-child { border-bottom: none; }

.nav-item {
  display: flex; align-items: center; gap: 8px;
  padding: 9px 16px; cursor: pointer; font-size: 13px; color: #a0a0b0;
  transition: background 0.15s;
}
.nav-item:hover { background: #252540; color: #e0e0e0; }
.nav-item.active { color: #409eff; background: rgba(64,158,255,0.1); }
.nav-item .arrow { margin-left: auto; font-size: 10px; transition: transform 0.2s; }
.nav-item .arrow.open { transform: rotate(90deg); }

.nav-sub-item {
  padding: 6px 16px 6px 36px; cursor: pointer; font-size: 12px; color: #808090;
  display: flex; align-items: center; justify-content: space-between;
}
.nav-sub-item:hover { background: #252540; color: #c0c0d0; }
.nav-sub-item.active { color: #409eff; background: rgba(64,158,255,0.08); }
.nav-sub-item .badge {
  font-size: 10px; background: #0f0f1a; color: #9090a0;
  padding: 1px 6px; border-radius: 8px;
}

.sidebar-footer { padding: 10px; border-top: 1px solid #2a2a4a; }
.task-hint { font-size: 11px; color: #888; margin-top: 4px; text-align: center; }
.el-main { padding: 20px; overflow-y: auto; }

/* 右键菜单 */
.context-menu { position: fixed; z-index: 9999; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 150px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
.ctx-item { padding: 9px 18px; cursor: pointer; font-size: 13px; color: #c0c0d0; }
.ctx-item:hover { background: #3a3a5a; color: #fff; }
.ctx-item.danger { color: #f56c6c; }
.ctx-item.danger:hover { background: #4a2020; }
</style>
