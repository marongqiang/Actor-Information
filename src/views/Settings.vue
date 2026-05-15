<template>
  <div class="settings-page">
    <h2>设置</h2>

    <div class="section">
      <h3>115 网盘登录</h3>
      <div v-if="!loggedIn">
        <el-tabs v-model="loginMethod" type="border-card" style="background: transparent;">
          <el-tab-pane label="扫码登录" name="qrcode">
            <el-button type="primary" @click="startLogin" :loading="loginLoading" style="margin-bottom: 12px;">
              {{ loginLoading ? '等待扫码...' : '获取二维码' }}
            </el-button>
            <div v-if="qrcodeUrl" style="margin-top: 12px; text-align: center;">
              <p>请使用 115 手机 App 扫描二维码：</p>
              <img :src="qrcodeUrl" style="width: 200px; border-radius: 8px;" />
              <p style="color: #888; margin-top: 8px;">状态: {{ loginStatusText }}</p>
            </div>
            <div v-if="qrError" style="margin-top: 12px;">
              <el-alert :title="qrError" type="error" :closable="false" />
              <p style="color: #888; margin-top: 8px; font-size: 13px;">
                扫码登录可能因115 API变更而失败，建议使用「Cookie登录」方式
              </p>
            </div>
          </el-tab-pane>
          <el-tab-pane label="Cookie登录（推荐）" name="cookie">
            <p style="color: #888; margin-bottom: 8px; font-size: 13px;">
              从浏览器登录115网盘后，在开发者工具(F12) → Application/存储 → Cookies → 复制所有Cookie值粘贴到下方
            </p>
            <el-input
              v-model="cookieInput"
              type="textarea"
              :rows="4"
              placeholder="粘贴完整的Cookie字符串，例如：UID=xxx; CID=xxx; SEID=xxx; ..."
              style="margin-bottom: 12px;"
            />
            <el-button type="primary" @click="loginByCookie" :loading="cookieLoading">
              {{ cookieLoading ? '验证中...' : 'Cookie登录' }}
            </el-button>
          </el-tab-pane>
        </el-tabs>
      </div>
      <div v-else>
        <el-tag type="success">已登录</el-tag>
        <el-button type="danger" @click="doLogout" style="margin-left: 12px;">退出登录</el-button>
      </div>
    </div>

    <div class="section">
      <h3>刮削设置</h3>
      <el-form label-width="140px">
        <el-form-item label="刮削数据源">
          <el-checkbox-group v-model="scrapeSources">
            <el-checkbox value="tmdb" label="TMDB" />
            <el-checkbox value="douban" label="豆瓣" />
            <el-checkbox value="javbus" label="JavBus" />
            <el-checkbox value="javdb" label="JavDB" />
            <el-checkbox value="fanza" label="Fanza" />
          </el-checkbox-group>
        </el-form-item>
        <el-form-item label="TMDB API Key">
          <el-input v-model="tmdbApiKey" type="password" show-password style="width: 320px;" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="saveScrapeSettings">保存刮削设置</el-button>
        </el-form-item>
      </el-form>
    </div>

    <div class="section">
      <h3>应用设置</h3>
      <el-form label-width="140px">
        <el-form-item label="主题">
          <el-radio-group v-model="theme">
            <el-radio value="dark">暗色</el-radio>
            <el-radio value="light">亮色</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="海报尺寸">
          <el-radio-group v-model="posterSize">
            <el-radio value="small">小</el-radio>
            <el-radio value="medium">中</el-radio>
            <el-radio value="large">大</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="窗口标题">
          <el-input v-model="privacyTitle" style="width: 320px;" />
        </el-form-item>
        <el-form-item label="外部播放器路径">
          <el-input v-model="externalPlayer" placeholder="如: C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe" style="width: 400px;" />
        </el-form-item>
        <el-form-item label="播放链接续期间隔">
          <el-input-number v-model="refreshInterval" :min="30" :max="600" /> 分钟
        </el-form-item>
        <el-form-item label="开机自启">
          <el-switch v-model="autoStart" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="saveAppSettings">保存应用设置</el-button>
        </el-form-item>
      </el-form>
    </div>

    <div class="section">
      <h3>演员管理</h3>
      <el-form label-width="180px">
        <el-form-item label="演员文件夹路径">
          <el-input v-model="actorBaseDir" placeholder="D:\Media Library\Actor Information\picture" style="width: 400px;" />
        </el-form-item>
        <el-form-item label="刮削后自动创建演员">
          <el-switch v-model="autoCreateActors" />
        </el-form-item>
        <el-form-item label="新增演员待审核">
          <el-switch v-model="actorPendingReview" />
        </el-form-item>
        <el-form-item label="允许重命名演员文件夹">
          <el-switch v-model="allowRenameFolder" />
        </el-form-item>
        <el-form-item label="合并时自动合并文件夹">
          <el-switch v-model="mergeAutoFolders" />
        </el-form-item>
        <el-form-item label="合并文件命名模式">
          <el-input v-model="mergeFilePattern" style="width: 200px;" />
        </el-form-item>
        <el-form-item label="合并默认模拟运行">
          <el-switch v-model="mergeDryRun" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="saveActorSettings">保存演员设置</el-button>
        </el-form-item>
      </el-form>
    </div>

    <div class="section">
      <h3>数据管理</h3>
      <el-button @click="exportList">导出影片列表</el-button>
      <el-button type="danger" @click="clearCache">清除图片缓存</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

const loggedIn = ref(false)
const loginLoading = ref(false)
const qrcodeUrl = ref('')
const loginUid = ref('')
const loginStatusText = ref('')
const qrError = ref('')
let loginTimer: ReturnType<typeof setInterval> | null = null

const loginMethod = ref('cookie')
const cookieInput = ref('')
const cookieLoading = ref(false)

const scrapeSources = ref<string[]>([])
const tmdbApiKey = ref('')
const theme = ref('dark')
const posterSize = ref('medium')
const privacyTitle = ref('智能网盘影视库')
const externalPlayer = ref('')
const refreshInterval = ref(240)
const autoStart = ref(false)

// Actor folder settings (1.2.0)
const actorBaseDir = ref('')
const autoCreateActors = ref(true)
const actorPendingReview = ref(true)
const allowRenameFolder = ref(false)
const mergeAutoFolders = ref(true)
const mergeFilePattern = ref('{name}_{index}{ext}')
const mergeDryRun = ref(true)

async function startLogin() {
  loginLoading.value = true
  qrError.value = ''
  try {
    const result: any = await invoke('login_qrcode')
    qrcodeUrl.value = result.qrcode_url
    loginUid.value = result.uid
    loginTimer = setInterval(checkLoginStatus, 2000)
  } catch (e: any) {
    const msg = e.message || e
    qrError.value = '获取二维码失败: ' + msg
    ElMessage.error(qrError.value)
    loginLoading.value = false
  }
}

async function loginByCookie() {
  if (!cookieInput.value.trim()) {
    ElMessage.warning('请先粘贴Cookie')
    return
  }
  cookieLoading.value = true
  try {
    await invoke('login_cookie_direct', { cookie: cookieInput.value.trim() })
    loggedIn.value = true
    ElMessage.success('Cookie登录成功')
  } catch (e: any) {
    ElMessage.error('Cookie验证失败: ' + (e.message || e))
  } finally {
    cookieLoading.value = false
  }
}

async function checkLoginStatus() {
  try {
    const result: any = await invoke('login_status', { uid: loginUid.value })
    const statusMap: Record<string, string> = { waiting: '等待扫码', scanned: '已扫码，请确认', authorized: '已授权', expired: '已过期' }
    loginStatusText.value = statusMap[result.status] || result.status
    if (result.status === 'authorized' && result.cookie) {
      await invoke('login_cookie', { cookie: result.cookie })
      loggedIn.value = true
      loginLoading.value = false
      if (loginTimer) clearInterval(loginTimer)
      ElMessage.success('登录成功')
    } else if (result.status === 'expired') {
      loginLoading.value = false
      if (loginTimer) clearInterval(loginTimer)
      ElMessage.warning('二维码已过期，请重新获取')
    }
  } catch { /* ignore polling errors */ }
}

async function doLogout() {
  await invoke('logout')
  loggedIn.value = false
  ElMessage.success('已退出登录')
}

async function saveScrapeSettings() {
  await invoke('set_config', { key: 'scrape_sources', value: JSON.stringify(scrapeSources.value) })
  if (tmdbApiKey.value) {
    await invoke('set_secure_config', { key: 'tmdb_api_key', value: tmdbApiKey.value })
  }
  ElMessage.success('刮削设置已保存')
}

async function saveAppSettings() {
  await invoke('set_config', { key: 'theme', value: theme.value })
  await invoke('set_config', { key: 'poster_size', value: posterSize.value })
  await invoke('set_config', { key: 'privacy_title', value: privacyTitle.value })
  await invoke('set_config', { key: 'external_player', value: externalPlayer.value })
  await invoke('set_config', { key: 'playback_refresh_interval', value: String(refreshInterval.value) })
  await invoke('set_config', { key: 'auto_start', value: String(autoStart.value) })
  ElMessage.success('应用设置已保存')
}

async function exportList() {
  const path = await invoke('export_list')
  ElMessage.success('已导出到: ' + path)
}

async function saveActorSettings() {
  await invoke('set_config', { key: 'local_actor_base_dir', value: actorBaseDir.value })
  await invoke('set_config', { key: 'auto_create_actors_from_scrape', value: String(autoCreateActors.value) })
  await invoke('set_config', { key: 'actor_pending_review', value: String(actorPendingReview.value) })
  await invoke('set_config', { key: 'allow_app_rename_actor_folders', value: String(allowRenameFolder.value) })
  await invoke('set_config', { key: 'actor_merge_auto_merge_folders', value: String(mergeAutoFolders.value) })
  await invoke('set_config', { key: 'actor_merge_file_naming_pattern', value: mergeFilePattern.value })
  await invoke('set_config', { key: 'actor_merge_dry_run', value: String(mergeDryRun.value) })
  ElMessage.success('演员设置已保存')
}

async function clearCache() {
  ElMessage.info('缓存清理功能待实现')
}

onMounted(async () => {
  try {
    loggedIn.value = await invoke('check_token')
    const sources: string | null = await invoke('get_config', { key: 'scrape_sources' })
    if (sources) scrapeSources.value = JSON.parse(sources)
    theme.value = (await invoke('get_config', { key: 'theme' }) as string) || 'dark'
    posterSize.value = (await invoke('get_config', { key: 'poster_size' }) as string) || 'medium'
    privacyTitle.value = (await invoke('get_config', { key: 'privacy_title' }) as string) || '智能网盘影视库'
    externalPlayer.value = (await invoke('get_config', { key: 'external_player' }) as string) || ''
    const interval: string | null = await invoke('get_config', { key: 'playback_refresh_interval' })
    if (interval) refreshInterval.value = parseInt(interval)
    const auto: string | null = await invoke('get_config', { key: 'auto_start' })
    autoStart.value = auto === 'true'

    // Load actor settings
    actorBaseDir.value = (await invoke('get_config', { key: 'local_actor_base_dir' }) as string) || ''
    autoCreateActors.value = ((await invoke('get_config', { key: 'auto_create_actors_from_scrape' }) as string) || '1') === '1'
    actorPendingReview.value = ((await invoke('get_config', { key: 'actor_pending_review' }) as string) || '1') === '1'
    allowRenameFolder.value = ((await invoke('get_config', { key: 'allow_app_rename_actor_folders' }) as string) || '0') === '1'
    mergeAutoFolders.value = ((await invoke('get_config', { key: 'actor_merge_auto_merge_folders' }) as string) || '1') === '1'
    mergeFilePattern.value = (await invoke('get_config', { key: 'actor_merge_file_naming_pattern' }) as string) || '{name}_{index}{ext}'
    mergeDryRun.value = ((await invoke('get_config', { key: 'actor_merge_dry_run' }) as string) || '1') === '1'
  } catch { /* config may not exist yet */ }
})
</script>

<style scoped>
.settings-page { max-width: 700px; }
h2 { margin-bottom: 20px; }
.section {
  margin-bottom: 20px; padding: 16px; background: #1a1a2e; border-radius: 8px;
}
.section h3 { font-size: 16px; margin-bottom: 12px; border-bottom: 1px solid #2a2a4a; padding-bottom: 8px; }
</style>
