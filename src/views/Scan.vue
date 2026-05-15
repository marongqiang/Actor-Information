<template>
  <div class="scan-page" v-loading.fullscreen.lock="fullscreenLoading" element-loading-text="扫描中，请稍候...">
    <h2>扫描管理</h2>

    <div class="section">
      <h3>115 网盘根目录</h3>
      <el-table :data="store.roots" style="width: 100%" border resizable stripe size="small">
        <el-table-column prop="cid" label="目录 ID" width="180" show-overflow-tooltip />
        <el-table-column prop="name" label="名称" show-overflow-tooltip />
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="openDir(row)">浏览</el-button>
            <el-button size="small" type="primary" @click="startScan(row)">扫描</el-button>
          </template>
        </el-table-column>
      </el-table>
      <p v-if="!store.roots.length" style="text-align: center; color: #888; padding: 24px;">请先在设置中登录115网盘</p>
    </div>

    <div class="section" v-if="currentDir">
      <h3>当前目录: {{ currentDir.name }}</h3>
      <div class="scan-options">
        <el-radio-group v-model="scanMode" size="small">
          <el-radio value="incremental">增量扫描</el-radio>
          <el-radio value="full">全量扫描</el-radio>
        </el-radio-group>
        <span>深度：</span>
        <el-input-number v-model="scanDepth" :min="1" :max="10" size="small" style="width: 80px;" />
        <el-button type="primary" @click="runScan" :loading="scanLoading" size="small">
          {{ scanLoading ? '扫描中...' : '开始扫描' }}
        </el-button>
      </div>
      <el-table :data="store.files" style="width: 100%; margin-top: 10px;" border resizable stripe size="small"
        v-loading="fileLoading" element-loading-text="加载文件列表...">
        <el-table-column prop="name" label="文件名" min-width="200" show-overflow-tooltip sortable="custom" />
        <el-table-column label="大小" width="100" sortable="custom">
          <template #default="{ row }">{{ formatSize(row.size) }}</template>
        </el-table-column>
        <el-table-column prop="is_dir" label="类型" width="70">
          <template #default="{ row }">{{ row.is_dir ? '📁' : '🎬' }}</template>
        </el-table-column>
        <el-table-column label="修改时间" width="160">
          <template #default="{ row }">{{ new Date(row.update_time * 1000).toLocaleString() }}</template>
        </el-table-column>
      </el-table>
    </div>

    <div class="section">
      <h3>任务列表</h3>
      <el-table :data="taskStore.tasks" style="width: 100%" border resizable stripe size="small">
        <el-table-column prop="id" label="任务 ID" width="120" show-overflow-tooltip />
        <el-table-column prop="type" label="类型" width="80" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="statusType(row.status)" size="small">{{ row.status }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="进度" width="180">
          <template #default="{ row }">
            <el-progress :percentage="row.progress" :stroke-width="6" :status="row.status === 'failed' ? 'exception' : undefined" />
          </template>
        </el-table-column>
        <el-table-column prop="error" label="错误信息" min-width="120" show-overflow-tooltip />
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="taskStore.resumeTask(row.id)" v-if="row.status === 'paused'">继续</el-button>
            <el-button size="small" type="danger" @click="taskStore.cancelTask(row.id)" v-if="row.status !== 'completed'">取消</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useScanStore } from '@/stores/scan'
import { useTaskStore } from '@/stores/task'
import { ElMessage } from 'element-plus'

const store = useScanStore()
const taskStore = useTaskStore()
const currentDir = ref<{ cid: string; name: string } | null>(null)
const scanMode = ref('incremental')
const scanDepth = ref(5)
const scanLoading = ref(false)
const fileLoading = ref(false)
const fullscreenLoading = ref(false)

function formatSize(bytes: number) {
  if (!bytes) return '-'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0; let size = bytes
  while (size > 1024 && i < units.length - 1) { size /= 1024; i++ }
  return size.toFixed(1) + ' ' + units[i]
}

function statusType(s: string) {
  return { running: 'warning', completed: 'success', failed: 'danger', pending: 'info', paused: '' }[s] || 'info'
}

async function openDir(dir: { cid: string; name: string }) {
  currentDir.value = dir
  fileLoading.value = true
  try { await store.getFiles(dir.cid) } catch (e: any) { ElMessage.error('加载失败: ' + (e.message || e)) }
  finally { fileLoading.value = false }
}

async function startScan(dir: { cid: string; name: string }) {
  currentDir.value = dir
  fileLoading.value = true
  try { await store.getFiles(dir.cid) } catch (e: any) { ElMessage.error('加载失败: ' + (e.message || e)) }
  finally { fileLoading.value = false }
}

async function runScan() {
  if (!currentDir.value) return
  scanLoading.value = true
  fullscreenLoading.value = true
  try {
    const result: any = await store.scanDirectory(currentDir.value.cid, scanDepth.value, scanMode.value)
    ElMessage.success(`扫描完成: 新增 ${result.new}，更新 ${result.updated}`)
    await taskStore.fetchPendingTasks()
  } catch (e: any) {
    ElMessage.error('扫描失败: ' + (e.message || e))
  } finally {
    scanLoading.value = false
    fullscreenLoading.value = false
  }
}

onMounted(async () => {
  await store.listRoot()
  await taskStore.fetchPendingTasks()
})
</script>

<style scoped>
.scan-page { }
h2 { font-size: 18px; margin-bottom: 16px; }
.section { margin-bottom: 20px; padding: 14px; background: #1a1a2e; border-radius: 8px; }
.section h3 { font-size: 15px; margin-bottom: 10px; }
.scan-options { display: flex; gap: 10px; align-items: center; margin: 10px 0; }
</style>
