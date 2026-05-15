<template>
  <div class="scan-page">
    <h2>扫描管理</h2>

    <!-- 步骤一：选择目录 -->
    <div class="section">
      <div class="step-header">
        <el-tag type="primary" size="small">步骤一</el-tag>
        <h3>选择要扫描的网盘目录</h3>
        <el-button size="small" @click="refreshRoots" :loading="loadingRoots">刷新</el-button>
      </div>

      <!-- 面包屑导航 -->
      <div class="breadcrumb" v-if="breadcrumbs.length">
        <el-button size="small" text @click="navToRoot">根目录</el-button>
        <template v-for="(b, i) in breadcrumbs" :key="b.cid">
          <span class="sep">/</span>
          <el-button size="small" text @click="navTo(b, i)">{{ b.name }}</el-button>
        </template>
      </div>

      <!-- 手动输入CID（快捷跳转） -->
      <div style="margin-top: 8px; display: flex; gap: 8px;">
        <el-input v-model="manualCid" placeholder="粘贴CID直接跳转目录..." size="small" style="flex: 1;" />
        <el-button size="small" @click="loadDir(manualCid); manualCid=''">跳转</el-button>
      </div>

      <!-- 目录/文件列表（固定高度+滚动） -->
      <div style="max-height: 340px; overflow: auto; margin-top: 8px;">
      <el-table :data="currentList" style="width: 100%;" border resizable stripe size="small"
        v-loading="loadingDirs" @selection-change="onDirSelect">
        <el-table-column type="selection" width="36" :selectable="isDir" />
        <el-table-column label="名称" min-width="260" show-overflow-tooltip sortable="custom">
          <template #default="{ row }">
            <span :class="{ 'is-file': !row.is_dir }" style="cursor: pointer;" @click="row.is_dir ? enterDir(row) : undefined">
              {{ row.is_dir ? '📁' : '🎬' }} {{ row.name }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="大小" width="90">
          <template #default="{ row }">{{ row.is_dir ? '-' : formatSize(row.size) }}</template>
        </el-table-column>
        <el-table-column label="修改时间" width="160">
          <template #default="{ row }">{{ row.update_time ? new Date(row.update_time * 1000).toLocaleString() : '-' }}</template>
        </el-table-column>
      </el-table>
      <p v-if="!currentList.length && !loadingDirs" style="text-align: center; color: #888; padding: 20px;">
        {{ store.roots.length ? '目录为空' : '请先在设置页登录115网盘' }}
      </p>
      </div>
    </div>

    <!-- 步骤二：扫描设置 -->
    <div class="section">
      <div class="step-header">
        <el-tag type="success" size="small">步骤二</el-tag>
        <h3>扫描设置</h3>
      </div>
      <div class="settings-row">
        <div class="setting-item">
          <label>已选目录</label>
          <span class="setting-val">
            <el-tag v-for="d in selectedDirs" :key="d.cid" closable size="small" @close="removeSelected(d)" style="margin: 2px;">
              {{ d.name }}
            </el-tag>
            <span v-if="!selectedDirs.length" style="color: #666;">未选择（将扫描当前目录及子目录）</span>
          </span>
        </div>
        <div class="setting-item">
          <label>扫描模式</label>
          <el-radio-group v-model="scanMode" size="small">
            <el-radio value="incremental">增量（仅新增/变更）</el-radio>
            <el-radio value="full">全量（重新扫描全部）</el-radio>
          </el-radio-group>
        </div>
        <div class="setting-item">
          <label>子目录深度</label>
          <el-input-number v-model="scanDepth" :min="1" :max="10" size="small" style="width: 100px;" />
          <span style="color: #888; font-size: 12px; margin-left: 8px;">1=仅当前目录，5=5层子目录</span>
        </div>
      </div>
    </div>

    <!-- 步骤三：执行 -->
    <div class="section">
      <div class="step-header">
        <el-tag type="warning" size="small">步骤三</el-tag>
        <h3>执行扫描</h3>
      </div>
      <div style="display: flex; gap: 12px; align-items: center; flex-wrap: wrap;">
        <el-button type="primary" size="large" @click="runScan" :loading="scanLoading" :disabled="!canScan">
          {{ scanLoading ? '扫描中...' : '开始扫描' }}
        </el-button>
        <el-checkbox v-model="autoScrape">扫描后自动刮削</el-checkbox>
      </div>

      <!-- 扫描结果 -->
      <div v-if="lastResult" style="margin-top: 12px; padding: 10px; background: #252540; border-radius: 6px;">
        <span style="color: #67c23a;">✓ 扫描完成</span>
        <span style="margin-left: 16px;">总计: {{ lastResult.total }}</span>
        <span style="margin-left: 16px; color: #67c23a;">新增: {{ lastResult.new }}</span>
        <span style="margin-left: 16px; color: #e6a23c;">更新: {{ lastResult.updated }}</span>
        <span style="margin-left: 16px; color: #f56c6c;">隐藏: {{ lastResult.deleted }}</span>
        <el-button size="small" text type="primary" style="margin-left: 16px;" @click="$router.push('/')">去海报墙查看 →</el-button>
      </div>
    </div>

    <!-- 扫描进度弹窗 -->
    <el-dialog v-model="scanDialogVisible" :title="scanDone ? '扫描完成' : '扫描中...'" width="400px"
      :close-on-click-modal="false" :show-close="scanDone" @close="scanDialogVisible = false">
      <div style="text-align: center; padding: 10px;">
        <!-- 大号计数 -->
        <div v-if="scanStats.files > 0" style="font-size: 48px; font-weight: bold; color: #409eff; margin: 12px 0;">
          {{ scanStats.files }}
        </div>
        <div v-else style="margin: 20px 0;">
          <el-icon class="is-loading" :size="36"><Loading /></el-icon>
        </div>
        <p style="font-size: 16px; color: #e0e0e0; margin-top: 8px;">{{ scanStatusText }}</p>
        <div v-if="scanDone" style="margin-top: 20px; text-align: left; background: #252540; border-radius: 6px; padding: 12px;">
          <p style="color: #67c23a;">✓ 新增: {{ scanStats.new }} 部</p>
          <p style="color: #e6a23c;">⟳ 更新: {{ scanStats.updated }} 部</p>
        </div>
      </div>
      <template #footer v-if="scanDone">
        <el-button @click="scanDialogVisible = false">关闭</el-button>
        <el-button type="primary" @click="scanDialogVisible = false; $router.push('/')">去海报墙查看</el-button>
      </template>
    </el-dialog>

    <!-- 历史任务 -->
    <div class="section" v-if="taskStore.tasks.length">
      <h3>扫描历史</h3>
      <el-table :data="taskStore.tasks" style="width: 100%" border resizable stripe size="small">
        <el-table-column prop="id" label="任务ID" width="100" show-overflow-tooltip />
        <el-table-column prop="status" label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="statusType(row.status)" size="small">{{ row.status }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="进度" width="160">
          <template #default="{ row }"><el-progress :percentage="row.progress" :stroke-width="6" /></template>
        </el-table-column>
        <el-table-column prop="created_at" label="时间" width="160">
          <template #default="{ row }">{{ new Date(row.created_at * 1000).toLocaleString() }}</template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, reactive } from 'vue'
import { useScanStore } from '@/stores/scan'
import { useTaskStore } from '@/stores/task'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { Loading } from '@element-plus/icons-vue'
import type { FileItem } from '@/types'

const store = useScanStore()
const taskStore = useTaskStore()

// Navigation
const breadcrumbs = ref<{ cid: string; name: string }[]>([])
const currentCid = ref('0')
const manualCid = ref('')
const loadingRoots = ref(false)
const loadingDirs = ref(false)
const scanLoading = ref(false)

// Progress dialog
const scanDialogVisible = ref(false)
const scanDone = ref(false)
const scanStatusText = ref('')
const scanStats = reactive({ files: 0, new: 0, updated: 0 })

// Selection
const selectedDirs = ref<{ cid: string; name: string }[]>([])
const scanMode = ref('incremental')
const scanDepth = ref(5)
const autoScrape = ref(false)
const lastResult = ref<any>(null)

const canScan = computed(() => currentCid.value !== '0' || selectedDirs.value.length > 0)

// All items in current view (dirs + files)
const currentList = computed(() => store.files)

function isDir(row: FileItem) { return row.is_dir }

async function refreshRoots() {
  loadingRoots.value = true
  try {
    await store.listRoot()
    currentCid.value = '0'
    breadcrumbs.value = []
    await loadDir('0')
  } catch (e: any) {
    ElMessage({ message: '加载失败: ' + (e.message || e) + '\n查看日志: 软件目录\\logs\\app.log', type: 'error', duration: 10000, showClose: true })
  } finally { loadingRoots.value = false }
}

async function loadDir(cid: string) {
  loadingDirs.value = true
  try { await store.getFiles(cid); currentCid.value = cid } catch (e: any) {
    ElMessage({ message: '加载失败: ' + (e.message||e) + '\n请重新登录获取新Cookie后重试\n日志: exe目录\\logs\\app.log', type: 'error', duration: 10000, showClose: true })
  } finally { loadingDirs.value = false }
}

async function enterDir(dir: FileItem) {
  breadcrumbs.value.push({ cid: dir.cid, name: dir.name })
  await loadDir(dir.cid)
}

function navToRoot() {
  breadcrumbs.value = []
  currentCid.value = '0'
  loadDir('0')
}

function navTo(_item: any, index: number) {
  breadcrumbs.value = breadcrumbs.value.slice(0, index + 1)
  currentCid.value = breadcrumbs.value[index]?.cid || '0'
  loadDir(currentCid.value)
}

function onDirSelect(rows: FileItem[]) {
  selectedDirs.value = rows.filter(r => r.is_dir).map(r => ({ cid: r.cid, name: r.name }))
}

function removeSelected(dir: { cid: string; name: string }) {
  selectedDirs.value = selectedDirs.value.filter(d => d.cid !== dir.cid)
}

async function runScan() {
  scanLoading.value = true
  // Open progress dialog
  scanDialogVisible.value = true; scanDone.value = false
  scanStatusText.value = '正在扫描网盘目录...'
  scanStats.files = 0; scanStats.new = 0; scanStats.updated = 0

    try {
    let totalNew = 0, totalUpdated = 0, totalAll = 0
    const dirsToScan = selectedDirs.value.length ? selectedDirs.value : [{ cid: currentCid.value, name: '' }]
    for (const dir of dirsToScan) {
      scanStatusText.value = `扫描: ${dir.name || '当前目录'}...`
      const result: any = await store.scanDirectory(dir.cid, scanDepth.value, scanMode.value)
      totalNew += result.new; totalUpdated += result.updated; totalAll += result.total
      scanStats.files = totalAll; scanStats.new = totalNew; scanStats.updated = totalUpdated
      scanStatusText.value = `已扫描到影片数量: ${totalAll}`
    }
    scanDone.value = true
    scanStatusText.value = `扫描完成，共发现 ${totalAll} 部影片`
    lastResult.value = { total: totalAll, new: totalNew, updated: totalUpdated, deleted: 0 }
  } catch (e: any) {
    scanDialogVisible.value = false
    ElMessage({ message: '扫描失败: ' + (e.message || e) + '\n日志: 软件目录\\logs\\app.log', type: 'error', duration: 8000, showClose: true })
  } finally {
    scanLoading.value = false
  }
}

function formatSize(bytes: number) {
  if (!bytes) return '-'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0; let size = bytes
  while (size > 1024 && i < units.length - 1) { size /= 1024; i++ }
  return size.toFixed(1) + ' ' + units[i]
}

function statusType(s: string) {
  return { running: 'warning', completed: 'success', failed: 'danger', pending: 'info' }[s] || 'info'
}

onMounted(async () => {
  await refreshRoots()
  await taskStore.fetchPendingTasks()
})
</script>

<style scoped>
.scan-page { max-width: 900px; }
h2 { font-size: 18px; margin-bottom: 12px; }
.section { margin-bottom: 18px; padding: 14px; background: #1a1a2e; border-radius: 8px; }
.step-header { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
.step-header h3 { font-size: 15px; }
.breadcrumb { display: flex; align-items: center; gap: 2px; font-size: 13px; margin-bottom: 6px; }
.breadcrumb .sep { color: #555; }
.is-file { color: #808090; cursor: default; }
.settings-row { display: flex; flex-direction: column; gap: 12px; }
.setting-item { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.setting-item label { font-size: 13px; color: #9090a0; min-width: 80px; }
.setting-val { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; }
</style>
