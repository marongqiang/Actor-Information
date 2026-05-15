<template>
  <div class="actress-table-page">
    <div class="toolbar">
      <h2>演员表格</h2>
      <el-input v-model="search" placeholder="搜索演员..." clearable style="width: 200px" @change="doSearch" />
      <el-switch v-model="showPending" active-text="待审核" inactive-text="全部" @change="doSearch" />
      <el-button type="success" size="small" @click="scanFolder">扫描本地文件夹</el-button>
      <el-button size="small" @click="detectDup">检测重复</el-button>
    </div>

    <el-table :data="store.actresses" v-loading="store.loading" style="width: 100%" border>
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="name" label="姓名" width="140" sortable>
        <template #default="{ row }">
          <span :style="{ color: row.is_pending ? '#e6a23c' : '' }">{{ row.name }}</span>
          <el-tag v-if="row.is_pending" type="warning" size="small" style="margin-left: 4px;">审</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="头像" width="70">
        <template #default="{ row }">
          <img v-if="row.avatar_local" :src="assetUrl(row.avatar_local)" style="width: 46px; height: 62px; object-fit: cover; border-radius: 4px;" />
        </template>
      </el-table-column>
      <el-table-column prop="local_folder_name" label="本地文件夹" width="120" />
      <el-table-column prop="source" label="来源" width="90" />
      <el-table-column prop="debut_year" label="出道年份" width="90" sortable />
      <el-table-column prop="height" label="身高" width="70" sortable />
      <el-table-column label="三围" width="120">
        <template #default="{ row }">
          <span v-if="row.bust">{{ row.bust }}-{{ row.waist }}-{{ row.hip }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="cup" label="罩杯" width="70" />
      <el-table-column prop="letter" label="首字母" width="70" />
      <el-table-column prop="movie_count" label="作品" width="70" sortable />
      <el-table-column label="操作" width="280" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="showAliases(row)">别名</el-button>
          <el-button v-if="row.is_pending" size="small" type="success" @click="confirmActor(row, true)">确认</el-button>
          <el-button v-if="row.is_pending" size="small" type="danger" @click="confirmActor(row, false)">拒绝</el-button>
          <el-button size="small" type="warning" @click="showMergeDialog(row)">合并</el-button>
          <el-button v-if="row.local_folder_name" size="small" @click="syncFolder(row)">同步</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      v-if="store.total > 20"
      v-model:current-page="page"
      :page-size="20"
      :total="store.total"
      layout="prev, pager, next"
      @current-change="onPageChange"
      style="margin-top: 20px; justify-content: center;"
    />

    <!-- 别名对话框 -->
    <el-dialog v-model="aliasDialog" title="演员别名" width="400px">
      <p><strong>{{ selectedActress?.name }}</strong> 的别名：</p>
      <el-tag v-for="a in aliases" :key="a" style="margin: 4px">{{ a }}</el-tag>
      <p v-if="!aliases.length" style="color: #888">暂无别名</p>
      <div style="margin-top: 16px; display: flex; gap: 8px;">
        <el-input v-model="newAlias" placeholder="新别名" size="small" />
        <el-button size="small" type="primary" @click="addAlias">添加</el-button>
      </div>
    </el-dialog>

    <!-- 合并对话框 -->
    <el-dialog v-model="mergeDialog" title="合并演员" width="500px">
      <p>将 <strong>{{ selectedActress?.name }}</strong> (ID: {{ selectedActress?.id }}) 合并到目标演员：</p>
      <el-select v-model="targetActressId" filterable placeholder="搜索目标演员..." style="width: 100%; margin: 12px 0;">
        <el-option v-for="a in store.actresses.filter(x => x.id !== selectedActress?.id)" :key="a.id" :label="`${a.name} (ID: ${a.id})`" :value="a.id" />
      </el-select>
      <div style="margin: 8px 0;">
        <el-checkbox v-model="mergeOptions.mergeFolders">合并文件夹</el-checkbox>
        <el-checkbox v-model="mergeOptions.dryRun">模拟运行（不实际修改）</el-checkbox>
      </div>
      <p style="margin: 4px 0;">冲突策略：</p>
      <el-radio-group v-model="mergeOptions.conflictPolicy">
        <el-radio value="rename">重命名</el-radio>
        <el-radio value="skip">跳过</el-radio>
        <el-radio value="overwrite">覆盖</el-radio>
      </el-radio-group>
      <div v-if="mergeResult" style="margin-top: 12px; padding: 8px; background: #1a1a2e; border-radius: 4px;">
        <p v-if="mergeResult.success" style="color: #67c23a">合并成功</p>
        <p v-else style="color: #f56c6c">{{ mergeResult.error }}</p>
        <p v-if="mergeResult.movedFiles.length">移动文件: {{ mergeResult.movedFiles.length }}</p>
        <p v-if="mergeResult.conflicts.length">冲突: {{ mergeResult.conflicts.length }}</p>
      </div>
      <template #footer>
        <el-button @click="mergeDialog = false">取消</el-button>
        <el-button type="primary" @click="executeMerge">执行合并</el-button>
      </template>
    </el-dialog>

    <!-- 重复检测对话框 -->
    <el-dialog v-model="dupDialog" title="检测到的重复演员" width="500px">
      <el-table :data="store.duplicates" style="width: 100%">
        <el-table-column prop="id1" label="演员1" width="80" />
        <el-table-column prop="id2" label="演员2" width="80" />
        <el-table-column prop="similarity" label="相似度">
          <template #default="{ row }">
            <el-progress :percentage="Math.round(row.similarity * 100)" :stroke-width="6" />
          </template>
        </el-table-column>
      </el-table>
      <p v-if="!store.duplicates.length" style="color: #888; text-align: center;">未发现重复</p>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useActressStore } from '@/stores/actress'
import { convertFileSrc } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import type { ActressItem, MergeOptions, MergeResult } from '@/types'

const store = useActressStore()
const search = ref('')
const showPending = ref(false)
const page = ref(1)

const aliasDialog = ref(false)
const selectedActress = ref<ActressItem | null>(null)
const aliases = ref<string[]>([])
const newAlias = ref('')

const mergeDialog = ref(false)
const targetActressId = ref<number | null>(null)
const mergeOptions = ref<MergeOptions>({ mergeFolders: true, conflictPolicy: 'rename', dryRun: true })
const mergeResult = ref<MergeResult | null>(null)

const dupDialog = ref(false)

function assetUrl(path: string) {
  return convertFileSrc(path)
}

function doSearch() {
  page.value = 1
  store.fetchPaginated(1, 20, search.value || undefined, undefined, undefined, !showPending.value)
}

function onPageChange(p: number) {
  page.value = p
  store.fetchPaginated(p, 20, search.value || undefined, undefined, undefined, !showPending.value)
}

async function showAliases(row: ActressItem) {
  selectedActress.value = row
  aliases.value = await store.getAliases(row.id)
  aliasDialog.value = true
}

async function addAlias() {
  if (!newAlias.value.trim() || !selectedActress.value) return
  await store.addAlias(selectedActress.value.id, newAlias.value.trim())
  aliases.value = await store.getAliases(selectedActress.value.id)
  newAlias.value = ''
}

async function confirmActor(row: ActressItem, accepted: boolean) {
  await store.confirmActor(row.id, accepted)
  ElMessage.success(accepted ? '已确认' : '已拒绝')
  doSearch()
}

function showMergeDialog(row: ActressItem) {
  selectedActress.value = row
  targetActressId.value = null
  mergeResult.value = null
  mergeDialog.value = true
}

async function executeMerge() {
  if (!selectedActress.value || !targetActressId.value) {
    ElMessage.warning('请选择目标演员')
    return
  }
  try {
    mergeResult.value = await store.mergeActressesWithOptions(
      selectedActress.value.id,
      targetActressId.value,
      mergeOptions.value,
    )
    if (mergeResult.value?.success) ElMessage.success('合并完成')
  } catch (e: any) {
    ElMessage.error('合并失败: ' + (e.message || e))
  }
}

async function syncFolder(row: ActressItem) {
  await store.syncWithLocalFolder(row.id)
  ElMessage.success('同步完成')
}

async function scanFolder() {
  try {
    const result = await store.scanLocalFolder()
    ElMessage.success(`扫描完成: 新增 ${result.added} / 总计 ${result.total}`)
    doSearch()
  } catch (e: any) {
    ElMessage.error('扫描失败: ' + (e.message || e))
  }
}

async function detectDup() {
  try {
    await store.detectDuplicates()
    dupDialog.value = true
  } catch (e: any) {
    ElMessage.error('检测失败: ' + (e.message || e))
  }
}

onMounted(() => {
  store.fetchPaginated(1, 20)
})
</script>

<style scoped>
.actress-table-page { }
.toolbar {
  display: flex; gap: 12px; align-items: center;
  margin-bottom: 20px; padding: 12px; background: #1a1a2e; border-radius: 8px;
}
.toolbar h2 { flex: 1; font-size: 18px; }
</style>
