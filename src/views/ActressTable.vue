<template>
  <div class="actress-table-page">
    <div class="toolbar">
      <h2>演员表格</h2>
      <el-input v-model="search" placeholder="搜索..." clearable style="width: 160px" size="small" @input="onSearchInput" @clear="doSearch()" />
      <el-button-group size="small">
        <el-button :type="showPendingOnly === undefined ? 'primary' : ''" @click="showPendingOnly = undefined; doSearch()">全部</el-button>
        <el-button :type="showPendingOnly === true ? 'warning' : ''" @click="showPendingOnly = true; doSearch()">待审核</el-button>
        <el-button :type="showPendingOnly === false ? 'success' : ''" @click="showPendingOnly = false; doSearch()">已确认</el-button>
      </el-button-group>
      <span style="color:#f56c6c;font-size:11px;">DEBUG: search="{{ search }}" page={{ page }} total={{ store.total }}</span>
      <el-button type="success" size="small" @click="scanFolder" :loading="scanning">扫描本地</el-button>
      <el-button size="small" @click="detectDup">检测重复</el-button>
      <el-button size="small" type="danger" @click="deleteAll">全部删除</el-button>
      <span v-if="selectedRows.length" class="batch-actions">
        已选 {{ selectedRows.length }} 项
        <el-button size="small" type="warning" @click="batchMerge">合并</el-button>
        <el-button size="small" type="danger" @click="batchDelete">删除</el-button>
      </span>
    </div>

    <!-- 筛选条件展示 -->
    <div class="filter-bar" v-if="search || showPendingOnly !== undefined || filterGroupId">
      <span class="filter-label">筛选:</span>
      <el-tag v-if="search" closable size="small" @close="search = ''; doSearch()">搜索: {{ search }}</el-tag>
      <el-tag v-if="showPendingOnly === true" closable size="small" type="warning" @close="showPendingOnly = undefined; doSearch()">待审核</el-tag>
      <el-tag v-if="showPendingOnly === false" closable size="small" type="success" @close="showPendingOnly = undefined; doSearch()">已确认</el-tag>
      <el-tag v-if="filterGroupId" closable size="small" @close="clearGroupFilter()">分组筛选</el-tag>
    </div>

    <div class="table-wrapper">
      <el-table
        ref="tableRef"
        :data="store.actresses"
        v-loading="store.loading"
        style="min-width: 1200px"
        max-height="calc(100vh - 180px)"
        border stripe resizable
        :default-sort="{ prop: 'name', order: 'ascending' }"
        @selection-change="onSelectionChange"
        @sort-change="onSortChange"
        @cell-dblclick="onCellDblClick"
        show-overflow-tooltip
      >
        <el-table-column type="selection" width="40" />
        <el-table-column prop="name" label="姓名" width="160" sortable="custom">
          <template #default="{ row }">
            <span class="name-cell">{{ row.name }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="aliases" label="别名" width="160" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_aliases`">
              <el-input v-model="editValue" size="small" @blur="saveCell(row, 'aliases')" @keyup.enter="saveCell(row, 'aliases')" ref="editInputRef" />
            </template>
            <span v-else class="editable-cell">{{ row._aliases?.join(', ') || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="source" label="来源" width="70" sortable="custom">
          <template #default="{ row }">{{ row.source === 'local_folder' ? '本地' : '网络' }}</template>
        </el-table-column>
        <el-table-column prop="local_folder_name" label="图片目录" width="160" sortable="custom" show-overflow-tooltip />
        <el-table-column prop="is_pending" label="待审核" width="75" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_is_pending`">
              <el-switch v-model="editBool" size="small" @change="saveCell(row, 'is_pending')" />
            </template>
            <span v-else class="editable-cell" :style="{ color: row.is_pending ? '#e6a23c' : '#67c23a' }">
              {{ row.is_pending ? '是' : '否' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="debut_year" label="出道年" width="85" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_debut_year`">
              <el-input v-model="editValue" size="small" class="edit-input" @blur="saveCell(row, 'debut_year')" @keyup.enter="saveCell(row, 'debut_year')" />
            </template>
            <span v-else class="editable-cell">{{ row.debut_year || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="height" label="身高" width="75" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_height`">
              <el-input v-model="editValue" size="small" class="edit-input" @blur="saveCell(row, 'height')" @keyup.enter="saveCell(row, 'height')" />
            </template>
            <span v-else class="editable-cell">{{ row.height || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="bust" label="胸围" width="70" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_bust`">
              <el-input v-model="editValue" size="small" class="edit-input" @blur="saveCell(row, 'bust')" @keyup.enter="saveCell(row, 'bust')" />
            </template>
            <span v-else class="editable-cell">{{ row.bust || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="waist" label="腰围" width="70" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_waist`">
              <el-input v-model="editValue" size="small" class="edit-input" @blur="saveCell(row, 'waist')" @keyup.enter="saveCell(row, 'waist')" />
            </template>
            <span v-else class="editable-cell">{{ row.waist || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="hip" label="臀围" width="70" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_hip`">
              <el-input v-model="editValue" size="small" class="edit-input" @blur="saveCell(row, 'hip')" @keyup.enter="saveCell(row, 'hip')" />
            </template>
            <span v-else class="editable-cell">{{ row.hip || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="cup" label="罩杯" width="75" sortable="custom">
          <template #default="{ row }">
            <template v-if="editingCell === `${row.id}_cup`">
              <el-input v-model="editValue" size="small" @blur="saveCell(row, 'cup')" @keyup.enter="saveCell(row, 'cup')" />
            </template>
            <span v-else class="editable-cell">{{ row.cup || '-' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="movie_count" label="作品数" width="80" sortable="custom" />
      </el-table>
    </div>

    <div class="table-footer">
      <el-pagination v-if="store.total > pageSize" v-model:current-page="page" :page-size="pageSize" :total="store.total"
        layout="prev, pager, next" @current-change="onPageChange" background size="small" />
      <el-select v-model="pageSize" size="small" style="width: 100px; margin-left: 12px;" @change="onPageSizeChange">
        <el-option :value="20" label="20条/页" />
        <el-option :value="50" label="50条/页" />
        <el-option :value="100" label="100条/页" />
      </el-select>
    </div>

    <!-- 合并对话框 -->
    <el-dialog v-model="mergeDialog" title="合并演员" width="500px">
      <p>将 <strong>{{ selectedActress?.name }}</strong> 合并到目标演员：</p>
      <el-select v-model="targetActressId" filterable placeholder="搜索目标..." style="width: 100%; margin: 12px 0;" size="small">
        <el-option v-for="a in store.actresses.filter(x => x.id !== selectedActress?.id)" :key="a.id" :label="`${a.name} (ID: ${a.id})`" :value="a.id" />
      </el-select>
      <div style="margin: 8px 0;">
        <el-checkbox v-model="mergeOpts.mergeFolders">合并文件夹</el-checkbox>
        <el-checkbox v-model="mergeOpts.dryRun">模拟运行</el-checkbox>
      </div>
      <el-radio-group v-model="mergeOpts.conflictPolicy" size="small">
        <el-radio value="rename">重命名</el-radio><el-radio value="skip">跳过</el-radio><el-radio value="overwrite">覆盖</el-radio>
      </el-radio-group>
      <div v-if="mergeResult" style="margin-top: 10px; padding: 8px; background: #252540; border-radius: 4px;">
        <p :style="{ color: mergeResult.success ? '#67c23a' : '#f56c6c' }">{{ mergeResult.success ? '合并成功' : mergeResult.error }}</p>
      </div>
      <template #footer>
        <el-button @click="mergeDialog = false">取消</el-button>
        <el-button type="primary" @click="executeMerge">执行</el-button>
      </template>
    </el-dialog>

    <!-- 重复检测 -->
    <el-dialog v-model="dupDialog" title="重复演员" width="500px">
      <el-table :data="store.duplicates" size="small">
        <el-table-column prop="id1" label="ID1" width="70" /><el-table-column prop="id2" label="ID2" width="70" />
        <el-table-column prop="similarity" label="相似度">
          <template #default="{ row }"><el-progress :percentage="Math.round(row.similarity * 100)" :stroke-width="6" /></template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useActressStore } from '@/stores/actress'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { ActressItem, MergeOptions, MergeResult } from '@/types'

const route = useRoute()
const $router = useRouter()
const store = useActressStore()
const search = ref(store.tableSearch); const showPendingOnly = ref<boolean | undefined>(store.tableShowPending)
const page = ref(Number(route.query.page) || store.tablePage); const pageSize = ref(20)

// Save state to store
watch(search, (v) => { store.tableSearch = v })
watch(showPendingOnly, (v) => { store.tableShowPending = v })
watch(page, (v) => { store.tablePage = v; if (v > 1) $router.replace({ query: { ...route.query, page: v } }) })

// Search: simple debounce
let searchTimer: any = null
function onSearchInput() {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => { page.value = 1; fetchData(1); searchTimer = null }, 400)
}
const scanning = ref(false); const selectedRows = ref<ActressItem[]>([])

const filterGroupId = ref<number | undefined>()
let firstLoad = true
watch(() => route.query.group_id, (val) => {
  filterGroupId.value = val ? Number(val) : undefined
  doSearch(firstLoad ? false : true) // don't reset page on initial load (restore from store)
  firstLoad = false
}, { immediate: true })

// Inline editing (统一用文本输入)
const editingCell = ref('')
const editValue = ref('')
const editBool = ref(false)
const editInputRef = ref()

// Merge
const mergeDialog = ref(false); const selectedActress = ref<ActressItem | null>(null)
const targetActressId = ref<number | null>(null)
const mergeOpts = ref<MergeOptions>({ mergeFolders: true, conflictPolicy: 'rename', dryRun: true })
const mergeResult = ref<MergeResult | null>(null)
const dupDialog = ref(false)

function doSearch(resetPage = true) { if (resetPage) page.value = 1; fetchData(page.value) }
function onPageChange(p: number) { page.value = p; fetchData(p) }
function onPageSizeChange() { page.value = 1; fetchData(1) }
function fetchData(p: number) {
  store.fetchPaginated(p, pageSize.value, search.value || undefined, undefined, undefined, showPendingOnly.value, filterGroupId.value).then(fetchAliases)
}
function onSelectionChange(rows: ActressItem[]) { selectedRows.value = rows }
function onSortChange(sort: any) {
  if (sort.prop) {
    store.fetchPaginated(page.value, pageSize.value, search.value || undefined, sort.prop, sort.order, showPendingOnly.value, filterGroupId.value).then(fetchAliases)
  }
}
function clearGroupFilter() {
  filterGroupId.value = undefined
  doSearch()
}

async function fetchAliases() {
  for (const a of store.actresses) { try { (a as any)._aliases = await store.getAliases(a.id) } catch { (a as any)._aliases = [] } }
}

// ─── 双击编辑 ───

function onCellDblClick(row: any, col: any) {
  if (!col.property || col.property === 'name' || col.property === 'movie_count') return
  if (col.type === 'selection') return
  editingCell.value = `${row.id}_${col.property}`
  if (col.property === 'aliases') {
    editValue.value = row._aliases?.join(', ') || ''
  } else if (col.property === 'is_pending') {
    editBool.value = row.is_pending
  } else {
    editValue.value = row[col.property] != null ? String(row[col.property]) : ''
  }
  nextTick(() => editInputRef.value?.focus?.())
}

async function saveCell(row: any, field: string) {
  const cellKey = `${row.id}_${field}`
  if (editingCell.value !== cellKey) return
  if (field === 'aliases') {
    const parts = editValue.value.split(',').map((s: string) => s.trim()).filter(Boolean)
    const unique = [...new Set(parts)]
    if (unique.length !== parts.length) ElMessage.warning('别名中存在重复，已自动去重')
    row._aliases = unique
    const oldAliases = await store.getAliases(row.id)
    for (const a of unique) {
      if (!oldAliases.includes(a)) await store.addAlias(row.id, a)
    }
  } else if (field === 'is_pending') {
    row[field] = editBool.value
    await store.updateActress(row.id, { is_pending: editBool.value } as any)
  } else {
    // Numeric or text: parse number for numeric fields
    const numFields = ['debut_year', 'height', 'bust', 'waist', 'hip']
    const val = numFields.includes(field) ? parseInt(editValue.value) || 0 : editValue.value
    row[field] = val
    await store.updateActress(row.id, { [field]: val })
  }
  editingCell.value = ''
}

async function scanFolder() {
  scanning.value = true
  try { const r = await store.scanLocalFolder(); ElMessage.success(`扫描完成: 新增 ${r.added}`); doSearch() }
  catch (e: any) { ElMessage.error('扫描失败: ' + (e.message || e)) }
  finally { scanning.value = false }
}

async function detectDup() { await store.detectDuplicates(); dupDialog.value = true }

function batchMerge() {
  if (selectedRows.value.length < 2) { ElMessage.warning('至少选择2个'); return }
  selectedActress.value = selectedRows.value[0]; targetActressId.value = selectedRows.value[1].id; mergeDialog.value = true
}
async function executeMerge() {
  if (!selectedActress.value || !targetActressId.value) return
  mergeResult.value = await store.mergeActressesWithOptions(selectedActress.value.id, targetActressId.value, mergeOpts.value)
  if (mergeResult.value?.success) { ElMessage.success('合并完成'); doSearch(); mergeDialog.value = false }
}

async function batchDelete() {
  if (!selectedRows.value.length) return
  await ElMessageBox.confirm(`确定删除选中的 ${selectedRows.value.length} 位演员？`, '确认删除', { type: 'warning' })
  const count = await store.deleteActresses(selectedRows.value.map(r => r.id))
  ElMessage.success(`已删除 ${count} 位`); doSearch()
}

async function deleteAll() {
  await ElMessageBox.confirm('确定删除数据库中全部演员数据？此操作不可恢复！', '危险操作', { type: 'error', confirmButtonClass: 'el-button--danger' })
  const count = await invoke('delete_all_actresses')
  ElMessage.success(`已删除 ${count} 位演员`); doSearch()
}

onMounted(() => {
  // watch(immediate) 已在 setup 阶段触发了 doSearch()
})
</script>

<style scoped>
.actress-table-page { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.toolbar { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; padding: 10px 14px; background: #1a1a2e; border-radius: 8px; margin-bottom: 10px; flex-shrink: 0; }
.toolbar h2 { font-size: 16px; }
.batch-actions { color: #409eff; font-size: 12px; margin-left: auto; display: flex; align-items: center; gap: 8px; }
.table-wrapper { flex: 1; overflow: auto; }
.table-footer { flex-shrink: 0; display: flex; justify-content: center; padding: 10px 0; }
.name-cell { color: #000; font-weight: 600; }
.editable-cell { cursor: pointer; padding: 2px 4px; border-radius: 2px; display: block; }
.editable-cell:hover { background: #252540; }
:deep(.edit-input .el-input__inner) { text-align: center; }
</style>
