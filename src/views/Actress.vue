<template>
  <div class="actress-page">
    <div class="toolbar">
      <h2>演员库</h2>
      <el-input v-model="search" placeholder="搜索演员..." clearable style="width: 200px" size="small" @change="doSearch" />
      <el-button type="primary" size="small" @click="store.syncData()">同步数据</el-button>
    </div>

    <div v-if="store.loading" class="loading"><el-icon class="is-loading"><Loading /></el-icon> 加载中...</div>

    <div v-else class="actress-grid-wrapper">
    <div class="actress-grid">
      <div v-for="actress in store.actresses" :key="actress.id" class="actress-card"
          @contextmenu.prevent="onContextMenu($event, actress)">
        <div class="avatar-container" @click.stop="goDetail(actress.id)">
          <img v-if="actress.avatar_local" :src="imgSrc[actress.avatar_local] || ''" class="avatar-img" />
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
    </div>

    <div class="table-footer">
      <el-pagination v-if="store.total > pageSize" :current-page="page"
        :page-size="pageSize" :total="store.total" layout="prev, pager, next" @current-change="onPageChange" background size="small" />
      <el-select v-model="pageSize" size="small" style="width: 100px; margin-left: 12px;" @change="onPageSizeChange">
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
      </el-select>
    </div>

    <!-- 右键菜单 -->
    <div v-if="ctx.visible" class="context-menu" :style="{ left: ctx.x + 'px', top: ctx.y + 'px' }" @mouseleave="ctx.visible = false">
      <div class="ctx-submenu" @mouseenter="ctxSub = 'group'" @mouseleave="ctxSub = ''">
        <div class="ctx-item">📁 添加到分组 ▸</div>
        <div v-if="ctxSub === 'group'" class="sub-menu">
          <div v-for="g in actressGroups" :key="'ag_'+g.id" class="ctx-item" @click="addToGroupById(g.id)">{{ g.name }}</div>
          <div v-if="!actressGroups.length" class="ctx-item" style="color:#666;">暂无分组</div>
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useActressStore } from '@/stores/actress'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { Loading, UserFilled } from '@element-plus/icons-vue'
import type { ActressItem, ActressGroupItem } from '@/types'

const route = useRoute()
const router = useRouter()
const store = useActressStore()
const search = ref('')
const page = ref(1)
const pageSize = ref(20)

// Read group_id from URL for filtering
const filterGroupId = ref<number | undefined>()
watch(() => route.query.group_id, (val) => {
  filterGroupId.value = val ? Number(val) : undefined
  doSearch()
}, { immediate: true })

// Context menu
const ctx = reactive({ visible: false, x: 0, y: 0, actress: null as ActressItem | null })
const ctxSub = ref('')
const actressGroups = ref<ActressGroupItem[]>([])

// Image cache: path -> base64 data URL
const imgSrc = reactive<Record<string, string>>({})
async function preloadImages() {
  let count = 0
  for (const a of store.actresses) {
    if (a.avatar_local && !imgSrc[a.avatar_local]) {
      try { imgSrc[a.avatar_local] = await invoke('read_image_base64', { path: a.avatar_local.replace(/\\/g, '/') }); count++ } catch { imgSrc[a.avatar_local] = '' }
    }
  }
  console.log('preloadImages:', count, 'loaded, total actresses:', store.actresses.length)
}

async function doSearch() { page.value = 1; await store.fetchPaginated(1, pageSize.value, search.value || undefined, undefined, undefined, undefined, filterGroupId.value); preloadImages() }
async function onPageChange(p: number) { page.value = p; await store.fetchPaginated(p, pageSize.value, search.value || undefined, undefined, undefined, undefined, filterGroupId.value); preloadImages() }
async function onPageSizeChange() { page.value = 1; await store.fetchPaginated(1, pageSize.value, search.value || undefined, undefined, undefined, undefined, filterGroupId.value); preloadImages() }

function goDetail(id: number) { router.push(`/actress/${id}`) }

function onContextMenu(e: MouseEvent, actress: ActressItem) {
  ctx.visible = true; ctx.x = e.clientX; ctx.y = e.clientY; ctx.actress = actress
  // Fetch groups for the menu
  invoke('get_actress_groups').then(g => actressGroups.value = g as any)
}

async function addToGroupById(groupId: number) {
  if (!ctx.actress) return
  ctx.visible = false
  await invoke('add_actresses_to_group', { groupId, actressIds: [ctx.actress.id] })
  window.dispatchEvent(new CustomEvent('groups-changed'))
  ElMessage.success(`已添加到分组`)
}

onMounted(() => {
  // watch(immediate) 已在 setup 阶段用正确的 group_id 触发了 doSearch()
})
</script>

<style scoped>
.actress-page { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.actress-grid-wrapper { flex: 1; overflow-y: auto; }
.table-footer { flex-shrink: 0; display: flex; justify-content: center; align-items: center; padding: 10px 0; }
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
.avatar-container { width: 80px; height: 80px; border-radius: 50%; overflow: hidden; background: #2a2a4a; margin: 0 auto; }
.avatar-img { width: 100%; height: 100%; object-fit: cover; border-radius: 50%; }
.avatar-placeholder { display: flex; align-items: center; justify-content: center; height: 100%; color: #555; border-radius: 50%; }
.actress-info { padding: 8px; }
.actress-name { font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.actress-meta { font-size: 11px; color: #888; margin-top: 4px; display: flex; gap: 6px; flex-wrap: wrap; }
.context-menu { position: fixed; z-index: 9999; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 150px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
.ctx-item { padding: 8px 16px; cursor: pointer; font-size: 13px; color: #c0c0d0; white-space: nowrap; }
.ctx-item:hover { background: #3a3a5a; color: #fff; }
.ctx-submenu { position: relative; }
.sub-menu { position: absolute; left: 100%; top: 0; background: #252540; border: 1px solid #3a3a5a; border-radius: 4px; min-width: 140px; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
</style>
