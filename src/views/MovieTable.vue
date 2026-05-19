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
        <el-button size="small" type="primary" @click="batchScrape">刮削</el-button>
        <el-button size="small" type="danger" @click="batchHide">隐藏</el-button>
        <el-button size="small" @click="batchUnhide">取消隐藏</el-button>
      </span>
    </div>

      <el-table :data="movies" v-loading="loading"
        style="width:100%;" border stripe resizable
        max-height="calc(100vh - 180px)"
        :default-sort="{ prop: 'updated_at', order: 'descending' }"
        @selection-change="onSelectionChange"
        @sort-change="onSortChange"
        show-overflow-tooltip
      >
        <el-table-column type="selection" width="40" />
        <el-table-column prop="title" label="片名" width="180" sortable="custom" fixed="left" show-overflow-tooltip />
        <el-table-column prop="original_title" label="原名" width="160" sortable="custom" show-overflow-tooltip />
        <el-table-column label="中文名" width="180" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.chinese_name">{{ row.chinese_name }}</span>
            <span v-else-if="row.original_title" style="display:flex;align-items:center;gap:6px;">
              <span style="color:#666;font-size:11px;">无</span>
              <el-button size="small" text type="primary" @click.stop="translateAndSave(row)">翻译</el-button>
            </span>
            <span v-else style="color:#666;font-size:11px;">-</span>
          </template>
        </el-table-column>
        <el-table-column prop="year" label="年份" width="70" sortable="custom" />
        <el-table-column prop="rating" label="评分" width="70" sortable="custom">
          <template #default="{ row }">{{ row.rating ? '★'+row.rating.toFixed(1) : '-' }}</template>
        </el-table-column>
        <el-table-column prop="runtime" label="时长" width="75" sortable="custom">
          <template #default="{ row }">{{ row.runtime ? row.runtime+'分' : '-' }}</template>
        </el-table-column>
        <el-table-column prop="genre" label="类型" width="150" show-overflow-tooltip>
          <template #default="{ row }">{{ (row.genre || []).join(', ') }}</template>
        </el-table-column>
        <el-table-column prop="scrape_status" label="刮削" width="80" sortable="custom">
          <template #default="{ row }">
            <el-tag :type="['info','warning','success','danger'][row.scrape_status||0]" size="small">
              {{ ['未刮削','刮削中','已刮削','失败'][row.scrape_status||0] }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="刮削开始" width="140" sortable="custom" prop="scrape_started_at">
          <template #default="{ row }">{{ row.scrape_started_at ? new Date(row.scrape_started_at * 1000).toLocaleString() : '-' }}</template>
        </el-table-column>
        <el-table-column label="刮削结束" width="140" sortable="custom" prop="scrape_finished_at">
          <template #default="{ row }">{{ row.scrape_finished_at ? new Date(row.scrape_finished_at * 1000).toLocaleString() : '-' }}</template>
        </el-table-column>
        <el-table-column prop="scrape_error" label="失败原因" width="160" show-overflow-tooltip>
          <template #default="{ row }">
            <span :style="{ color: row.scrape_status === 3 ? '#f56c6c' : '#666' }">{{ row.scrape_error || '-' }}</span>
          </template>
        </el-table-column>
      </el-table>

    <div class="table-footer">
      <el-pagination v-if="total > pageSize" :current-page="page" :page-size="pageSize" :total="total"
        layout="prev, pager, next" @current-change="onPageChange" background size="small" />
      <el-select v-model="pageSize" size="small" style="width: 100px; margin-left: 12px;" @change="onPageSizeChange">
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
      </el-select>
    </div>

    <!-- 刮削进度弹窗 -->
    <el-dialog v-model="scrapeDialog" title="刮削进度" width="400px" :close-on-click-modal="false" :show-close="false">
      <div style="text-align: center; padding: 10px;">
        <el-progress :percentage="scrapePct" :stroke-width="12" :status="scrapeDone ? 'success' : undefined" />
        <p style="margin-top: 12px; color: #e0e0e0;">{{ scrapeText }}</p>
        <p v-if="scrapeDone" style="margin-top: 8px; color: #67c23a;">✓ 成功 {{ scrapeOk }} / 失败 {{ scrapeFail }}</p>
      </div>
      <template #footer v-if="scrapeDone">
        <el-button @click="scrapeDialog=false; doSearch()">关闭</el-button>
      </template>
    </el-dialog>
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

// Scrape progress
const scrapeDialog = ref(false)
const scrapePct = ref(0)
const scrapeDone = ref(false)
const scrapeText = ref('')
const scrapeOk = ref(0)
const scrapeFail = ref(0)

function formatSize(bytes: number) {
  if (!bytes) return '-'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0; let s = bytes
  while (s > 1024 && i < units.length - 1) { s /= 1024; i++ }
  return s.toFixed(1) + ' ' + units[i]
}

async function translateAndSave(row: MovieItem) {
  if (!row.original_title) return
  try {
    const cn = await invoke('translate_text', { text: row.original_title }) as string
    if (cn && cn !== row.original_title) {
      await invoke('set_movie_chinese_name', { fileId: row.file_id, chineseName: cn })
      row.chinese_name = cn
      ElMessage.success('翻译完成: ' + cn)
    }
  } catch (e: any) {
    ElMessage.error('翻译失败: ' + (e?.message || e))
  }
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

async function batchScrape() {
  if (!selectedRows.value.length) return
  const ids = selectedRows.value.map(r => r.file_id)
  scrapeDialog.value = true; scrapePct.value = 0; scrapeDone.value = false
  scrapeText.value = `正在刮削 ${ids.length} 部影片...`; scrapeOk.value = 0; scrapeFail.value = 0

  // Start scrape in background, poll progress
  const scrapePromise = invoke('scrape_batch', { fileIds: ids })
  const pollTimer = setInterval(async () => {
    try {
      const stats: any = await invoke('get_scrape_stats')
      const done = stats.success + stats.failed
      const selTotal = ids.length
      if (done > 0) {
        scrapePct.value = Math.round(done / selTotal * 100)
        scrapeOk.value = stats.success
        scrapeFail.value = stats.failed
        scrapeText.value = `[${done}/${selTotal}] 成功${stats.success} 失败${stats.failed}`
        doSearch() // refresh table to show per-row status changes
      }
    } catch { /* ignore poll errors */ }
  }, 1000)

  try {
    const result: any = await scrapePromise
    clearInterval(pollTimer)
    scrapePct.value = 100; scrapeDone.value = true
    scrapeOk.value = result.success; scrapeFail.value = result.failed
    scrapeText.value = `刮削完成: 成功${result.success} 失败${result.failed}`
    doSearch()
  } catch(e: any) {
    clearInterval(pollTimer)
    scrapeDialog.value = false
    ElMessage.error('刮削失败: ' + (e?.message || e))
  }
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
.table-footer { flex-shrink: 0; display: flex; justify-content: center; align-items: center; padding: 10px 0; }
</style>
