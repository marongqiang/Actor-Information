<template>
  <div class="movie-table-page">
    <div class="toolbar">
      <h2>影片表格</h2>
      <el-input v-model="search" placeholder="搜索..." clearable style="width: 180px" size="small" @change="doSearch" />
      <el-button-group size="small">
        <el-button :type="!showHidden ? 'primary' : ''" @click="showHidden = false; doSearch()">可见</el-button>
        <el-button :type="showHidden ? 'warning' : ''" @click="showHidden = true; doSearch()">已隐藏</el-button>
      </el-button-group>
      <span v-if="selectedRows.length" class="batch-actions">
        已选 {{ selectedRows.length }} 项
        <el-button size="small" type="danger" @click="batchHide">隐藏</el-button>
        <el-button size="small" @click="batchUnhide">取消隐藏</el-button>
      </span>
    </div>

    <div class="table-wrapper">
      <el-table :data="movies" v-loading="loading"
        style="min-width: 1500px" max-height="calc(100vh - 190px)" border stripe resizable
        :default-sort="{ prop: 'updated_at', order: 'descending' }"
        @selection-change="onSelectionChange"
        @sort-change="onSortChange"
        show-overflow-tooltip
      >
        <el-table-column type="selection" width="40" />
        <el-table-column prop="title" label="片名" width="180" sortable="custom" fixed="left" show-overflow-tooltip />
        <el-table-column prop="original_title" label="原名" width="160" sortable="custom" show-overflow-tooltip />
        <el-table-column prop="year" label="年份" width="70" sortable="custom" />
        <el-table-column prop="rating" label="评分" width="70" sortable="custom">
          <template #default="{ row }">{{ row.rating ? '★'+row.rating.toFixed(1) : '-' }}</template>
        </el-table-column>
        <el-table-column prop="runtime" label="时长" width="75" sortable="custom">
          <template #default="{ row }">{{ row.runtime ? row.runtime+'分' : '-' }}</template>
        </el-table-column>
        <el-table-column prop="director" label="导演" width="120" sortable="custom" show-overflow-tooltip />
        <el-table-column prop="genre" label="类型" width="150" show-overflow-tooltip>
          <template #default="{ row }">{{ (row.genre || []).join(', ') }}</template>
        </el-table-column>
        <el-table-column prop="file_name" label="文件名" width="200" sortable="custom" show-overflow-tooltip />
        <el-table-column label="大小" width="100" sortable="custom">
          <template #default="{ row }">{{ formatSize(row.file_size) }}</template>
        </el-table-column>
        <el-table-column prop="file_id" label="文件ID" width="160" sortable="custom" show-overflow-tooltip />
        <el-table-column label="修改时间" width="160" sortable="custom">
          <template #default="{ row }">{{ row.updated_at ? new Date(row.updated_at * 1000).toLocaleString() : '-' }}</template>
        </el-table-column>
        <el-table-column prop="is_favorite" label="收藏" width="70" sortable="custom">
          <template #default="{ row }">
            <el-tag :type="row.is_favorite ? 'warning' : 'info'" size="small">{{ row.is_favorite ? '是' : '否' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="group_names" label="所属分组" width="180" show-overflow-tooltip>
          <template #default="{ row }">{{ (row.group_names || []).join(', ') || '-' }}</template>
        </el-table-column>
        <el-table-column prop="is_hidden" label="隐藏" width="70" sortable="custom">
          <template #default="{ row }">
            <el-tag :type="row.is_hidden ? 'danger' : 'success'" size="small">{{ row.is_hidden ? '是' : '否' }}</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div class="table-footer">
      <el-pagination v-if="total > pageSize" :current-page="page" :page-size="pageSize" :total="total"
        layout="prev, pager, next" @current-change="onPageChange" background size="small" />
      <el-select v-model="pageSize" size="small" style="width: 100px; margin-left: 12px;" @change="onPageSizeChange">
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
      </el-select>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useLibraryStore } from '@/stores/library'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { MovieItem } from '@/types'

const route = useRoute()
const store = useLibraryStore()
const movies = ref<MovieItem[]>([])
const loading = ref(false)
const search = ref('')
const showHidden = ref(false)
const page = ref(1)
const pageSize = ref(20)
const total = ref(0)
const selectedRows = ref<MovieItem[]>([])
const sortProp = ref('updated_at')
const sortOrder = ref('desc')

function formatSize(bytes: number) {
  if (!bytes) return '-'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0; let s = bytes
  while (s > 1024 && i < units.length - 1) { s /= 1024; i++ }
  return s.toFixed(1) + ' ' + units[i]
}

function doSearch() { page.value = 1; fetchData() }
function onPageChange(p: number) { page.value = p; fetchData() }
function onPageSizeChange() { page.value = 1; fetchData() }

async function fetchData() {
  loading.value = true
  try {
    const result: any = await invoke('get_movies', {
      filters: { keyword: search.value || undefined, is_hidden: showHidden.value || undefined },
      sort: `${sortProp.value}_${sortOrder.value}`,
      page: page.value,
      pageSize: pageSize.value,
    })
    movies.value = result.movies || []
    total.value = result.total || 0
  } finally { loading.value = false }
}

function onSelectionChange(rows: MovieItem[]) { selectedRows.value = rows }

function onSortChange(sort: any) {
  if (sort.prop) { sortProp.value = sort.prop; sortOrder.value = sort.order || 'asc'; fetchData() }
}

async function batchHide() {
  if (!selectedRows.value.length) return
  await ElMessageBox.confirm(`确定隐藏选中的 ${selectedRows.value.length} 部影片？`, '确认', { type: 'warning' })
  await invoke('hide_movies', { fileIds: selectedRows.value.map(r => r.file_id) })
  ElMessage.success('已隐藏'); doSearch()
}

async function batchUnhide() {
  if (!selectedRows.value.length) return
  await invoke('unhide_movies', { fileIds: selectedRows.value.map(r => r.file_id) })
  ElMessage.success('已取消隐藏'); doSearch()
}

onMounted(() => { fetchData() })
</script>

<style scoped>
.movie-table-page { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.toolbar { flex-shrink: 0; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; padding: 10px 14px; background: #1a1a2e; border-radius: 8px; margin-bottom: 10px; }
.toolbar h2 { font-size: 16px; }
.batch-actions { color: #409eff; font-size: 12px; margin-left: auto; display: flex; align-items: center; gap: 8px; }
.table-wrapper { flex: 1; overflow: auto; }
.table-footer { flex-shrink: 0; display: flex; justify-content: center; align-items: center; padding: 10px 0; }
</style>
