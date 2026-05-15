<template>
  <div class="actress-table-page">
    <div class="toolbar">
      <h2>演员表格</h2>
      <el-input v-model="search" placeholder="搜索演员..." clearable style="width: 240px" @change="doSearch" />
    </div>

    <el-table :data="store.actresses" v-loading="store.loading" style="width: 100%" border>
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column prop="name" label="姓名" width="160" sortable />
      <el-table-column label="头像" width="80">
        <template #default="{ row }">
          <img v-if="row.avatar_local" :src="assetUrl(row.avatar_local)" style="width: 50px; height: 67px; object-fit: cover; border-radius: 4px;" />
        </template>
      </el-table-column>
      <el-table-column prop="debut_year" label="出道年份" width="100" sortable />
      <el-table-column prop="height" label="身高(cm)" width="100" sortable />
      <el-table-column label="三围" width="140">
        <template #default="{ row }">
          <span v-if="row.bust">{{ row.bust }}-{{ row.waist }}-{{ row.hip }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="cup" label="罩杯" width="80" />
      <el-table-column prop="letter" label="首字母" width="80" />
      <el-table-column prop="movie_count" label="作品数" width="80" sortable />
      <el-table-column label="操作" width="120">
        <template #default="{ row }">
          <el-button size="small" @click="showAliases(row)">别名</el-button>
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

    <el-dialog v-model="aliasDialog" title="演员别名" width="400px">
      <p><strong>{{ selectedActress?.name }}</strong> 的别名：</p>
      <el-tag v-for="a in aliases" :key="a" style="margin: 4px">{{ a }}</el-tag>
      <p v-if="!aliases.length" style="color: #888">暂无别名</p>
      <div style="margin-top: 16px; display: flex; gap: 8px;">
        <el-input v-model="newAlias" placeholder="新别名" size="small" />
        <el-button size="small" type="primary" @click="addAlias">添加</el-button>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useActressStore } from '@/stores/actress'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { ActressItem } from '@/types'

const store = useActressStore()
const search = ref('')
const page = ref(1)
const aliasDialog = ref(false)
const selectedActress = ref<ActressItem | null>(null)
const aliases = ref<string[]>([])
const newAlias = ref('')

function assetUrl(path: string) {
  return convertFileSrc(path)
}

function doSearch() {
  page.value = 1
  store.fetchPaginated(1, 20, search.value || undefined)
}

function onPageChange(p: number) {
  page.value = p
  store.fetchPaginated(p, 20, search.value || undefined)
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
