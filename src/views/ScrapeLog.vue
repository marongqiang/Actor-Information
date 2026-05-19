<template>
  <div class="scrape-log-page">
    <div class="toolbar">
      <h2>刮削日志</h2>
      <span style="color:#888;font-size:13px;">
        共 {{ logs.length }} 条 | 成功: {{ logs.filter(l=>l.status===2).length }} | 失败: {{ logs.filter(l=>l.status===3).length }} | 待刮削: {{ logs.filter(l=>l.status===1).length }} | 未刮削: {{ logs.filter(l=>l.status===0).length }}
      </span>
      <el-button size="small" @click="refresh" :loading="loading">刷新</el-button>
    </div>

    <el-table :data="logs" v-loading="loading" size="small" border stripe max-height="calc(100vh - 120px)"
      :default-sort="{ prop: 'status', order: 'ascending' }">
      <el-table-column prop="code" label="番号" width="160" sortable="custom" fixed="left" show-overflow-tooltip />
      <el-table-column prop="title" label="片名" min-width="200" sortable="custom" show-overflow-tooltip />
      <el-table-column label="状态" width="90" sortable="custom" prop="status">
        <template #default="{ row }">
          <el-tag :type="['info','warning','success','danger'][row.status]" size="small">
            {{ ['未刮削','待刮削','成功','失败'][row.status] }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="开始时间" width="160" sortable="custom" prop="started_at">
        <template #default="{ row }">{{ row.started_at ? new Date(row.started_at * 1000).toLocaleString() : '-' }}</template>
      </el-table-column>
      <el-table-column label="结束时间" width="160" sortable="custom" prop="finished_at">
        <template #default="{ row }">{{ row.finished_at ? new Date(row.finished_at * 1000).toLocaleString() : '-' }}</template>
      </el-table-column>
      <el-table-column prop="error" label="错误日志" min-width="200" show-overflow-tooltip>
        <template #default="{ row }">
          <span :style="{ color: row.status === 3 ? '#f56c6c' : '#666' }">{{ row.error || '-' }}</span>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ScrapeLogRow {
  file_id: string; title: string; code: string
  status: number; started_at: number | null; finished_at: number | null
  error: string | null
}

const logs = ref<ScrapeLogRow[]>([])
const loading = ref(false)

async function refresh() {
  loading.value = true
  try { logs.value = await invoke('get_scrape_logs') as ScrapeLogRow[] }
  catch { logs.value = [] }
  finally { loading.value = false }
}

onMounted(refresh)
</script>

<style scoped>
.scrape-log-page { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.toolbar { flex-shrink: 0; display: flex; gap: 16px; align-items: center; margin-bottom: 16px; flex-wrap: wrap; }
h2 { font-size: 18px; }
</style>
