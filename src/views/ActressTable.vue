<template>
  <div class="actress-table-page">
    <div class="toolbar">
      <h2>演员表格</h2>
      <el-input v-model="search" placeholder="搜索..." clearable style="width: 160px" size="small" @change="doSearch" />
      <el-switch v-model="showPending" active-text="待审核" inactive-text="全部" size="small" @change="doSearch" />
      <el-button type="success" size="small" @click="scanFolder" :loading="scanning">扫描本地</el-button>
      <el-button size="small" @click="detectDup">检测重复</el-button>
      <span v-if="selectedRows.length" class="batch-actions">
        已选 {{ selectedRows.length }} 项
        <el-button size="small" type="warning" @click="batchMerge">合并</el-button>
        <el-button size="small" type="danger" @click="batchDelete">删除</el-button>
      </span>
    </div>

    <div class="table-wrapper">
      <el-table
        ref="tableRef"
        :data="store.actresses"
        v-loading="store.loading"
        style="width: 100%"
        border stripe resizable
        :default-sort="{ prop: 'movie_count', order: 'descending' }"
        @selection-change="onSelectionChange"
        @sort-change="onSortChange"
        show-overflow-tooltip
      >
        <el-table-column type="selection" width="40" fixed="left" />
        <el-table-column prop="name" label="姓名" width="140" sortable="custom" fixed="left">
          <template #default="{ row }">
            <span :style="{ color: row.is_pending ? '#e6a23c' : '#e0e0e0' }">{{ row.name }}</span>
            <el-tag v-if="row.is_pending" type="warning" size="small" style="margin-left: 4px;">审</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="aliases" label="别名" min-width="100">
          <template #default="{ row }">
            <span v-if="row._aliases?.length">{{ row._aliases.join(', ') }}</span>
            <span v-else style="color: #555;">-</span>
          </template>
        </el-table-column>
        <el-table-column prop="source" label="来源" width="70">
          <template #default="{ row }">
            <el-tag :type="row.source === 'local_folder' ? 'success' : ''" size="small">{{ row.source === 'local_folder' ? '本地' : '网络' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="debut_year" label="出道年" width="75" sortable="custom" />
        <el-table-column prop="height" label="身高" width="65" sortable="custom" />
        <el-table-column label="三围" width="100">
          <template #default="{ row }"><span v-if="row.bust">{{ row.bust }}/{{ row.waist }}/{{ row.hip }}</span></template>
        </el-table-column>
        <el-table-column prop="cup" label="罩杯" width="60" />
        <el-table-column prop="movie_count" label="作品" width="60" sortable="custom" />
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="showAliases(row)">别名</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div class="table-footer">
      <el-pagination
        v-if="store.total > 20"
        v-model:current-page="page"
        :page-size="20" :total="store.total"
        layout="prev, pager, next" @current-change="onPageChange" background size="small"
      />
    </div>

    <!-- 别名对话框 -->
    <el-dialog v-model="aliasDialog" title="演员别名" width="400px">
      <p><strong>{{ selectedActress?.name }}</strong> 的别名：</p>
      <el-tag v-for="a in aliases" :key="a" style="margin: 4px" closable @close="removeAlias(a)">{{ a }}</el-tag>
      <p v-if="!aliases.length" style="color: #888">暂无别名</p>
      <div style="margin-top: 16px; display: flex; gap: 8px;">
        <el-input v-model="newAlias" placeholder="新别名" size="small" />
        <el-button size="small" type="primary" @click="addAlias">添加</el-button>
      </div>
    </el-dialog>

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
        <p v-if="mergeResult.movedFiles?.length">移动文件: {{ mergeResult.movedFiles.length }}</p>
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
      <p v-if="!store.duplicates.length" style="color: #888; text-align: center;">未发现重复</p>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useActressStore } from '@/stores/actress'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { ActressItem, MergeOptions, MergeResult } from '@/types'

const store = useActressStore()
const search = ref(''); const showPending = ref(false); const page = ref(1)
const scanning = ref(false); const selectedRows = ref<ActressItem[]>([])

const aliasDialog = ref(false); const selectedActress = ref<ActressItem | null>(null)
const aliases = ref<string[]>([]); const newAlias = ref('')

const mergeDialog = ref(false); const targetActressId = ref<number | null>(null)
const mergeOpts = ref<MergeOptions>({ mergeFolders: true, conflictPolicy: 'rename', dryRun: true })
const mergeResult = ref<MergeResult | null>(null)
const dupDialog = ref(false)

function doSearch() { page.value = 1; store.fetchPaginated(1, 20, search.value || undefined, undefined, undefined, !showPending.value) }
function onPageChange(p: number) { page.value = p; store.fetchPaginated(p, 20, search.value || undefined, undefined, undefined, !showPending.value) }
function onSelectionChange(rows: ActressItem[]) { selectedRows.value = rows }
function onSortChange(sort: any) { store.fetchPaginated(page.value, 20, search.value || undefined, sort.prop, sort.order) }

async function fetchAliases() {
  for (const a of store.actresses) { try { (a as any)._aliases = await store.getAliases(a.id) } catch { (a as any)._aliases = [] } }
}

async function showAliases(row: ActressItem) { selectedActress.value = row; aliases.value = await store.getAliases(row.id); aliasDialog.value = true }
async function addAlias() { if (!newAlias.value.trim() || !selectedActress.value) return; await store.addAlias(selectedActress.value.id, newAlias.value.trim()); aliases.value = await store.getAliases(selectedActress.value.id); newAlias.value = '' }
async function removeAlias(_: string) { ElMessage.info('删除别名待实现') }

async function scanFolder() { scanning.value = true; try { const r = await store.scanLocalFolder(); ElMessage.success(`扫描完成: 新增 ${r.added} / 总计 ${r.total}`); doSearch() } catch (e: any) { ElMessage.error('扫描失败: ' + (e.message || e)) } finally { scanning.value = false } }
async function detectDup() { await store.detectDuplicates(); dupDialog.value = true }

async function batchMerge() { if (selectedRows.value.length < 2) { ElMessage.warning('至少选择2个'); return }; selectedActress.value = selectedRows.value[0]; targetActressId.value = selectedRows.value[1].id; mergeDialog.value = true }
async function executeMerge() { if (!selectedActress.value || !targetActressId.value) { ElMessage.warning('请选择目标'); return }; mergeResult.value = await store.mergeActressesWithOptions(selectedActress.value.id, targetActressId.value, mergeOpts.value); if (mergeResult.value?.success) { ElMessage.success('合并完成'); doSearch() } }

async function batchDelete() {
  if (!selectedRows.value.length) return
  await ElMessageBox.confirm(`确定删除选中的 ${selectedRows.value.length} 位演员？`, '确认删除', { type: 'warning' })
  const ids = selectedRows.value.map(r => r.id)
  const count = await store.deleteActresses(ids)
  ElMessage.success(`已删除 ${count} 位演员`)
  doSearch()
}

onMounted(async () => { await store.fetchPaginated(1, 20); await fetchAliases() })
</script>

<style scoped>
.actress-table-page { display: flex; flex-direction: column; height: calc(100vh - 60px); }
.toolbar { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; padding: 10px 14px; background: #1a1a2e; border-radius: 8px; margin-bottom: 10px; flex-shrink: 0; }
.toolbar h2 { font-size: 16px; }
.batch-actions { color: #409eff; font-size: 12px; margin-left: auto; display: flex; align-items: center; gap: 8px; }
.table-wrapper { flex: 1; overflow: auto; }
.table-footer { flex-shrink: 0; display: flex; justify-content: center; padding: 12px 0; }
</style>
