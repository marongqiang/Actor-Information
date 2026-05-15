<template>
  <div id="app-container">
    <el-container>
      <el-aside width="200px" class="sidebar">
        <div class="logo" @click="$router.push('/')">
          <h2>智能网盘影视库</h2>
        </div>
        <el-menu
          :default-active="currentRoute"
          router
          background-color="#1a1a2e"
          text-color="#a0a0b0"
          active-text-color="#409eff"
        >
          <el-menu-item index="/">
            <el-icon><PictureFilled /></el-icon>
            <span>海报墙</span>
          </el-menu-item>
          <el-menu-item index="/favorites">
            <el-icon><StarFilled /></el-icon>
            <span>收藏影片</span>
          </el-menu-item>
          <el-menu-item index="/actress">
            <el-icon><UserFilled /></el-icon>
            <span>演员库</span>
          </el-menu-item>
          <el-menu-item index="/actress-table">
            <el-icon><Grid /></el-icon>
            <span>演员表格</span>
          </el-menu-item>
          <el-menu-item index="/scan">
            <el-icon><FolderOpened /></el-icon>
            <span>扫描管理</span>
          </el-menu-item>
          <el-menu-item index="/settings">
            <el-icon><Setting /></el-icon>
            <span>设置</span>
          </el-menu-item>
        </el-menu>
        <div class="sidebar-footer">
          <el-progress
            v-if="scanTask"
            :percentage="scanTask.progress"
            :status="scanTask.status === 'failed' ? 'exception' : undefined"
            :stroke-width="6"
          />
          <p v-if="scanTask" class="task-hint">扫描: {{ scanTask.progress }}%</p>
        </div>
      </el-aside>
      <el-main>
        <router-view />
      </el-main>
    </el-container>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { PictureFilled, UserFilled, Grid, FolderOpened, Setting, StarFilled } from '@element-plus/icons-vue'
import type { Task } from '@/types'

const route = useRoute()
const currentRoute = computed(() => route.path)
const scanTask = ref<Task | null>(null)

onMounted(async () => {
  await listen('scan-progress', (event: any) => {
    scanTask.value = {
      id: '',
      type: 'scan',
      status: 'running',
      progress: event.payload.percent || 0,
      created_at: 0,
      updated_at: 0,
    }
  })
})
</script>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: 'Microsoft YaHei', sans-serif; background: #0f0f1a; color: #e0e0e0; }
#app-container { height: 100vh; }
.el-container { height: 100%; }
.sidebar {
  background: #1a1a2e;
  display: flex; flex-direction: column;
  border-right: 1px solid #2a2a4a;
}
.logo { padding: 18px 14px; cursor: pointer; border-bottom: 1px solid #2a2a4a; }
.logo h2 { font-size: 15px; color: #409eff; text-align: center; }
.el-menu { border-right: none !important; flex: 1; }
.sidebar-footer { padding: 10px; border-top: 1px solid #2a2a4a; }
.task-hint { font-size: 11px; color: #888; margin-top: 4px; text-align: center; }
.el-main { padding: 20px; overflow-y: auto; }
</style>
