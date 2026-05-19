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
        <el-option :value="10" label="10条/页" />
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
        <el-option :value="200" label="200条/页" />
        <el-option :value="300" label="300条/页" />
      </el-select>
    </div>

    <!-- 刮削进度浮窗（右上角，不阻塞操作） -->
    <div v-if="scraping" class="scrape-float">
      <span>[{{ scrapeDone2 }}/{{ scrapeTotal }}] 成功{{ scrapeOk }} 失败{{ scrapeFail }}</span>
      <el-progress :percentage="scrapePct" :stroke-width="4" :status="scrapeDone ? 'success' : undefined" style="width:120px;" />
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

// Scrape progress (floating, non-blocking, persists across navigation)
const win = window as any
if (!win._scrapeState) win._scrapeState = { scraping: false, pct: 0, done: false, ok: 0, fail: 0, done2: 0, total: 0, timer: null as any }
const scrapeState = win._scrapeState
const scraping = ref(scrapeState.scraping)
const scrapePct = ref(scrapeState.pct)
const scrapeDone = ref(scrapeState.done)
const scrapeOk = ref(scrapeState.ok)
const scrapeFail = ref(scrapeState.fail)
const scrapeDone2 = ref(scrapeState.done2)
const scrapeTotal = ref(scrapeState.total)

function syncScrapeState() {
  Object.assign(scrapeState, {
    scraping: scraping.value, pct: scrapePct.value, done: scrapeDone.value,
    ok: scrapeOk.value, fail: scrapeFail.value, done2: scrapeDone2.value, total: scrapeTotal.value
  })
}

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
  scraping.value = true; scrapePct.value = 0; scrapeDone.value = false
  scrapeOk.value = 0; scrapeFail.value = 0; scrapeTotal.value = ids.length; scrapeDone2.value = 0

  // Fire-and-forget: don't await, let it run in background
  invoke('scrape_batch', { fileIds: ids }).then((result: any) => {
    scrapeOk.value = result.success; scrapeFail.value = result.failed
  }).catch((e: any) => {
    ElMessage.error('刮削失败: ' + (e?.message || e))
  })

  // Poll progress for selected batch only
  const pollTimer = setInterval(async () => {
    try {
      const stats: any = await invoke('get_batch_progress', { fileIds: ids })
      const done = stats.success + stats.failed
      scrapeDone2.value = done
      scrapeOk.value = stats.success
      scrapeFail.value = stats.failed
      scrapePct.value = stats.total > 0 ? Math.round(done / stats.total * 100) : 0
      syncScrapeState()
      if (stats.pending === 0 && done > 0) {
        // All done — close after 10 seconds
        scrapeDone.value = true; syncScrapeState()
        if (scrapeState.timer) clearTimeout(scrapeState.timer)
        scrapeState.timer = setTimeout(() => {
          scraping.value = false; scrapeDone.value = false
          syncScrapeState()
        }, 10000)
        clearInterval(pollTimer)
        doSearch()
      } else if (done > 0) {
        doSearch() // refresh table periodically
      }
    } catch { /* ignore */ }
  }, 1500)
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
.scrape-float { position: fixed; top: 12px; right: 20px; z-index: 9999;
  background: #252540; border: 1px solid #3a3a5a; border-radius: 6px;
  padding: 6px 14px; display: flex; align-items: center; gap: 12px;
  font-size: 13px; color: #e0e0e0; box-shadow: 0 2px 8px rgba(0,0,0,0.4); }
.table-footer { flex-shrink: 0; display: flex; justify-content: center; align-items: center; padding: 10px 0; }
</style>
