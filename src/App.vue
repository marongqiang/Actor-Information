<template>
  <div id="app-container">
    <el-container>
      <el-aside width="200px" class="sidebar">
        <div class="logo" @click="$router.push('/')"><h2>智能网盘影视库</h2></div>

        <nav class="nav-list">
          <!-- 海报墙 + 分组 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/' }"
              @contextmenu.prevent="openGroupMenu($event, 'poster')">
              <el-icon><PictureFilled /></el-icon><span @click="navTo('/')">海报墙</span>
              <el-icon class="arrow" :class="{ open: expanded.poster }" @click.stop="expanded.poster = !expanded.poster"><ArrowRight /></el-icon>
            </div>
            <template v-if="expanded.poster">
              <div v-for="g in posterGroups" :key="'pg_'+g.id" class="nav-sub-item"
                :class="{ active: currentRoute === '/' && route.query.group_id == String(g.id) }"
                @click.stop="navTo('/', {group_id: g.id})"
                @contextmenu.prevent.stop="onGroupItemCtx($event, 'poster', g.id, g.name)">{{ g.name }}<span class="badge">{{ g.movie_count || 0 }}</span></div>
            </template>
          </div>
          <!-- 收藏影片 + 分组 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/favorites' }"
              @contextmenu.prevent="openGroupMenu($event, 'favorite')">
              <el-icon><StarFilled /></el-icon><span @click.stop="navTo('/favorites')">收藏影片</span>
              <el-icon class="arrow" :class="{ open: expanded.favorites }" @click.stop="expanded.favorites = !expanded.favorites"><ArrowRight /></el-icon>
            </div>
            <template v-if="expanded.favorites">
              <div v-for="g in favGroups" :key="'fg_'+g.id" class="nav-sub-item"
                :class="{ active: currentRoute === '/favorites' && route.query.group_id == String(g.id) }"
                @click.stop="navTo('/favorites', {group_id: g.id})"
                @contextmenu.prevent.stop="onGroupItemCtx($event, 'favorite', g.id, g.name)">{{ g.name }}<span class="badge">{{ g.movie_count || 0 }}</span></div>
            </template>
          </div>
          <!-- 演员库 + 分组 -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/actress' }"
              @contextmenu.prevent="openGroupMenu($event, 'actress')">
              <el-icon><UserFilled /></el-icon><span @click.stop="navTo('/actress')">演员库</span>
              <el-icon class="arrow" :class="{ open: expanded.actress }" @click.stop="expanded.actress = !expanded.actress"><ArrowRight /></el-icon>
            </div>
            <template v-if="expanded.actress">
              <div v-for="g in actressGroups" :key="'ag_'+g.id" class="nav-sub-item"
                :class="{ active: currentRoute === '/actress' && route.query.group_id == String(g.id) }"
                @click.stop="navTo('/actress', {group_id: g.id})"
                @contextmenu.prevent.stop="onGroupItemCtx($event, 'actress', g.id, g.name)">{{ g.name }}<span class="badge">{{ g.member_count || 0 }}</span></div>
            </template>
          </div>
          <!-- 影片表格 + 演员表格（独立入口） -->
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/movie-table' }" @click="$router.push('/movie-table')">
              <el-icon><VideoCamera /></el-icon><span>影片表格</span></div>
          </div>
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/actress-table' }" @click="$router.push('/actress-table')">
              <el-icon><Grid /></el-icon><span>演员表格</span></div>
          </div>
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/scan' }" @click="$router.push('/scan')">
              <el-icon><FolderOpened /></el-icon><span>扫描管理</span></div>
          </div>
          <div class="nav-section">
            <div class="nav-item" :class="{ active: currentRoute === '/settings' }" @click="$router.push('/settings')">
              <el-icon><Setting /></el-icon><span>设置</span></div>
          </div>
        </nav>

        <div class="sidebar-footer">
          <el-progress v-if="scanTask" :percentage="scanTask.progress"
            :status="scanTask.status === 'failed' ? 'exception' : undefined" :stroke-width="6" />
          <p v-if="scanTask" class="task-hint">扫描: {{ scanTask.progress }}%</p>
        </div>
      </el-aside>
      <el-main><router-view /></el-main>
    </el-container>

    <!-- 右键菜单：入口 → 新增分组 -->
    <div v-if="ctx.visible && !ctx.groupId" class="context-menu" :style="{ left: ctx.x + 'px', top: ctx.y + 'px' }" @mouseleave="ctx.visible = false">
      <div class="ctx-item" @click="showAddGroup">➕ 新增分组</div>
    </div>
    <!-- 右键菜单：具体分组 → 重命名/删除 -->
    <div v-if="ctx.visible && ctx.groupId" class="context-menu" :style="{ left: ctx.x + 'px', top: ctx.y + 'px' }" @mouseleave="ctx.visible = false">
      <div class="ctx-item" @click="renameCtxGroup">✏️ 重命名</div>
      <div class="ctx-item danger" @click="deleteCtxGroup">🗑️ 删除</div>
    </div>

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
import { useRoute, useRouter } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import { PictureFilled, UserFilled, FolderOpened, Setting, StarFilled, ArrowRight, Grid, VideoCamera } from '@element-plus/icons-vue'
import type { Task, GroupItem, ActressGroupItem } from '@/types'

const route = useRoute()
const currentRoute = computed(() => route.path)
const currentFullPath = computed(() => route.fullPath)
const scanTask = ref<Task | null>(null)

const expanded = reactive({ poster: true, favorites: true, actress: true })
const posterGroups = ref<GroupItem[]>([])
const favGroups = ref<GroupItem[]>([])
const actressGroups = ref<ActressGroupItem[]>([])

type GroupCtx = 'poster' | 'favorite' | 'actress'
const ctx = reactive({ visible: false, x: 0, y: 0, type: 'poster' as GroupCtx, groupId: null as number | null, groupName: '' })

const groupFormDialog = ref(false); const groupFormTitle = ref(''); const groupFormName = ref('')
let groupFormMode: 'add' | 'rename' | 'delete' = 'add'; let editingGroupId: number | null = null

const $router = useRouter()

function navTo(path: string, query?: Record<string, any>) {
  const q = query || {}
  $router.push({ path, query: q })
  // Fallback: force reload via event if route didn't change
  window.dispatchEvent(new CustomEvent('nav-refresh', { detail: { path, query: q } }))
}

function openGroupMenu(e: MouseEvent, type: GroupCtx) {
  e.preventDefault(); ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.type = type; ctx.groupId = null; ctx.groupName = ''
}

function onGroupItemCtx(e: MouseEvent, type: GroupCtx, id: number, name: string) {
  e.preventDefault(); e.stopPropagation()
  ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.type = type; ctx.groupId = id; ctx.groupName = name
}

function showAddGroup() {
  ctx.visible = false; groupFormMode = 'add'; editingGroupId = null
  const label = { poster: '影片', favorite: '收藏', actress: '演员' }[ctx.type]
  groupFormTitle.value = `新建${label}分组`; groupFormName.value = ''; groupFormDialog.value = true
}

function renameCtxGroup() {
  ctx.visible = false
  groupFormMode = 'rename'; editingGroupId = ctx.groupId
  groupFormTitle.value = `重命名分组「${ctx.groupName}」`; groupFormName.value = ctx.groupName; groupFormDialog.value = true
}

async function deleteCtxGroup() {
  ctx.visible = false
  if (!ctx.groupId) return
  await ElMessageBox.confirm(`确定删除分组「${ctx.groupName}」？`, '确认删除', { type: 'warning' })
  if (ctx.type === 'actress') { await invoke('delete_actress_group', { groupId: ctx.groupId }) }
  else { await invoke('delete_group', { groupId: ctx.groupId }) }
  ElMessage.success('已删除'); await fetchGroups()
}

async function confirmGroupForm() {
  if (!groupFormName.value.trim()) { ElMessage.warning('请输入名称'); return }
  try {
    if (groupFormMode === 'add') {
      if (ctx.type === 'actress') {
        await invoke('create_actress_group', { name: groupFormName.value.trim() })
      } else {
        const gtype = ctx.type === 'favorite' ? 'favorite' : 'manual'
        await invoke('create_group', { name: groupFormName.value.trim(), groupType: gtype })
      }
      ElMessage.success('分组已创建')
    } else if (groupFormMode === 'rename' && editingGroupId) {
      if (ctx.type === 'actress') {
        await invoke('rename_actress_group', { groupId: editingGroupId, newName: groupFormName.value.trim() })
      } else {
        await invoke('rename_group', { groupId: editingGroupId, newName: groupFormName.value.trim() })
      }
      ElMessage.success('已重命名')
    }
    groupFormDialog.value = false; await fetchGroups()
  } catch (e: any) {
    ElMessage.error(e?.message || e || '操作失败')
  }
}

async function fetchGroups() {
  try {
    posterGroups.value = await invoke('get_groups', { category: 'manual' })
    favGroups.value = await invoke('get_groups', { category: 'favorite' })
    actressGroups.value = await invoke('get_actress_groups')
  } catch { /* ignore */ }
}

onMounted(async () => {
  await fetchGroups()
  window.addEventListener('groups-changed', () => fetchGroups())
  await listen('scan-progress', (event: any) => {
    scanTask.value = { id: '', type: 'scan', status: 'running', progress: event.payload.percent || 0, created_at: 0, updated_at: 0 }
  })
})
</script>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: 'Microsoft YaHei', sans-serif; background: #0f0f1a; color: #e0e0e0; }
#app-container { height: 100vh; }
.el-container { height: 100%; }
.sidebar { background: #1a1a2e; display: flex; flex-direction: column; border-right: 1px solid #2a2a4a; user-select: none; }
.logo { padding: 16px 14px; cursor: pointer; border-bottom: 1px solid #2a2a4a; }
.logo h2 { font-size: 15px; color: #409eff; text-align: center; }
.nav-list { flex: 1; overflow-y: auto; padding: 4px 0; }
.nav-section { padding: 2px 0; border-bottom: 1px solid #22223a; }
.nav-section:last-child { border-bottom: none; }
.nav-item { display: flex; align-items: center; gap: 8px; padding: 9px 16px; cursor: pointer; font-size: 13px; color: #a0a0b0; transition: background 0.15s; }
.nav-item:hover { background: #252540; color: #e0e0e0; }
.nav-item.active { color: #409eff; background: rgba(64,158,255,0.1); }
.nav-item .arrow { margin-left: auto; font-size: 10px; transition: transform 0.2s; }
.nav-item .arrow.open { transform: rotate(90deg); }
.nav-sub-item { padding: 6px 16px 6px 36px; cursor: pointer; font-size: 12px; color: #808090; display: flex; align-items: center; justify-content: space-between; }
.nav-sub-item:hover { background: #252540; color: #c0c0d0; }
.nav-sub-item.active { color: #409eff; background: rgba(64,158,255,0.08); }
.nav-sub-item .badge { font-size: 10px; background: #0f0f1a; color: #9090a0; padding: 1px 6px; border-radius: 8px; }
.sidebar-footer { padding: 10px; border-top: 1px solid #2a2a4a; }
.task-hint { font-size: 11px; color: #888; margin-top: 4px; text-align: center; }
.el-main { padding: 20px; overflow-y: auto; }
.context-menu { position: fixed; z-index: 9999; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 150px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
.ctx-item { padding: 9px 18px; cursor: pointer; font-size: 13px; color: #c0c0d0; }
.ctx-item:hover { background: #3a3a5a; color: #fff; }
.ctx-item.danger { color: #f56c6c; }
.ctx-item.danger:hover { background: #4a2020; }
</style>
