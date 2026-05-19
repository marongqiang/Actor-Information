<template>
  <div class="settings-page">
    <h2>设置</h2>

    <el-tabs v-model="activeTab" type="border-card">
      <!-- 刮削源 -->
      <el-tab-pane label="刮削源" name="scrape">
        <el-table :data="sourceTable" style="width:100%;" size="small" max-height="500" border resizable stripe>
              <el-table-column width="40">
                <template #header>
                  <el-checkbox :model-value="allChecked" :indeterminate="indeterminate" @change="toggleAllSources" />
                </template>
                <template #default="{ row }">
                  <el-checkbox :model-value="scrapeSources.includes(row.key)" @change="(v:boolean) => toggleSource(row.key, v)" />
                </template>
              </el-table-column>
              <el-table-column prop="name" label="名称" width="100" />
              <el-table-column prop="site" label="网站" show-overflow-tooltip />
              <el-table-column prop="type" label="类型" width="70" />
              <el-table-column label="已实现" width="70">
                <template #default="{ row }"><el-tag :type="row.done ? 'success' : 'info'" size="small">{{ row.done ? '是' : '否' }}</el-tag></template>
              </el-table-column>
              <el-table-column label="Token/Key" width="200">
                <template #default="{ row }">
                  <template v-if="row.key === 'tmdb'">
                    <el-input v-model="tmdbApiKey" type="password" show-password size="small" placeholder="TMDB API Key" />
                  </template>
                  <template v-else-if="row.key === 'deepseek'">
                    <el-input v-model="deepseekApiKey" type="password" show-password size="small" placeholder="DeepSeek API Key" />
                  </template>
                  <template v-else-if="row.key === 'jphoo'">
                    <el-input v-model="jphooSecret" type="password" show-password size="small" placeholder="secret" style="margin-bottom:2px;" />
                    <el-input v-model="jphooRefresh" size="small" placeholder="refreshtoken" style="margin-bottom:2px;" />
                    <el-input v-model="jphooGuestId" size="small" placeholder="guestid (UUID)" />
                  </template>
                  <span v-else style="color:#666;font-size:11px;">无需</span>
                </template>
              </el-table-column>
              <el-table-column label="备注" min-width="160">
                <template #default="{ row }">
                  <el-input v-model="row.remark" size="small" :placeholder="row.defaultRemark||''" />
                </template>
              </el-table-column>
            </el-table>
          <div style="margin-top:12px;">
            <el-button type="primary" @click="saveScrapeSettings">保存刮削设置</el-button>
          </div>
      </el-tab-pane>

      <!-- 网络代理 -->
      <el-tab-pane label="网络代理" name="proxy">
        <el-form label-width="120px" size="small">
          <el-form-item label="启用代理">
            <el-switch v-model="proxyEnabled" />
          </el-form-item>
          <el-form-item label="代理类型">
            <el-select v-model="proxyType" style="width: 160px;">
              <el-option label="HTTP" value="http" />
              <el-option label="SOCKS5" value="socks5" />
            </el-select>
          </el-form-item>
          <el-form-item label="代理地址">
            <el-input v-model="proxyHost" placeholder="127.0.0.1" style="width: 160px;" />
          </el-form-item>
          <el-form-item label="代理端口">
            <el-input-number v-model="proxyPort" :min="1" :max="65535" />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveProxySettings">保存代理设置</el-button>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- 缓存 -->
      <el-tab-pane label="缓存" name="cache">
        <el-form label-width="120px" size="small">
          <el-form-item label="图片缓存大小">
            <el-input-number v-model="cacheMaxSize" :min="128" :max="10240" /> MB
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveCacheSettings">保存缓存设置</el-button>
          </el-form-item>
        </el-form>
        <el-divider />
        <div class="danger-zone">
          <h4>数据管理</h4>
          <div style="display: flex; gap: 12px; margin-bottom: 8px;">
            <el-button @click="exportList">导出影片列表</el-button>
            <el-button type="danger" @click="clearCache">清除图片缓存</el-button>
            <el-button type="danger" @click="cleanGroups">清理所有分组</el-button>
          </div>
          <p style="font-size: 11px; color: #666; margin-top: 8px;">
            📋 日志文件: 软件目录\logs\app.log
          </p>
        </div>
      </el-tab-pane>

      <!-- 界面 -->
      <el-tab-pane label="界面" name="ui">
        <el-form label-width="140px" size="small">
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
          <el-form-item label="字体大小">
            <el-input-number v-model="fontSize" :min="10" :max="24" /> px
          </el-form-item>
          <el-form-item label="窗口标题">
            <el-input v-model="privacyTitle" style="width: 240px;" />
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
            <el-button type="primary" @click="saveUiSettings">保存界面设置</el-button>
          </el-form-item>
        </el-form>
      </el-tab-pane>

      <!-- 常规 -->
      <el-tab-pane label="常规" name="general">
        <!-- 115 登录 -->
        <h3>115 网盘登录</h3>
        <div v-if="!loggedIn" style="margin-bottom: 24px;">
          <el-tabs v-model="loginMethod" type="card" size="small">
            <el-tab-pane label="扫码登录" name="qrcode">
              <div style="padding: 12px 0;">
                <el-button type="primary" @click="startLogin" :loading="loginLoading" size="small">
                  {{ loginLoading ? '等待扫码...' : '获取二维码' }}
                </el-button>
                <div v-if="qrcodeUrl" style="margin-top: 12px; text-align: center;">
                  <p>请使用 115 手机 App 扫描二维码：</p>
                  <img :src="qrcodeUrl" style="width: 200px; border-radius: 8px;" />
                  <p style="color: #888; margin-top: 8px; font-size: 12px;">状态: {{ loginStatusText }}</p>
                </div>
                <div v-if="qrError" style="margin-top: 12px;">
                  <el-alert :title="qrError" type="error" :closable="false" />
                </div>
              </div>
            </el-tab-pane>
            <el-tab-pane label="Cookie登录（推荐）" name="cookie">
              <div style="padding: 12px 0;">
                <p style="color: #888; margin-bottom: 8px; font-size: 12px;">获取Cookie步骤：</p>
                <ol style="color: #888; font-size: 12px; margin-bottom: 8px; padding-left: 16px;">
                  <li>浏览器打开 <b>115.com</b> 并登录</li>
                  <li>按 <b>F12</b> → Application(应用程序) → Cookies → 115.com</li>
                  <li>复制 <b>UID、CID、SEID</b> 三个值，格式: <code>UID=xxx; CID=xxx; SEID=xxx</code></li>
                </ol>
                <el-input v-model="cookieInput" type="textarea" :rows="3" placeholder="UID=123456; CID=abcdef; SEID=xyz789" style="margin-bottom: 8px;" />
                <el-button type="primary" @click="loginByCookie" :loading="cookieLoading" size="small">
                  {{ cookieLoading ? '验证中...' : 'Cookie登录' }}
                </el-button>
                <p style="color: #888; font-size: 11px; margin-top: 4px;">仅需 UID/CID/SEID 三个字段即可</p>
              </div>
            </el-tab-pane>
          </el-tabs>
        </div>
        <div v-else style="margin-bottom: 24px;">
          <el-tag type="success">已登录</el-tag>
          <el-button type="danger" size="small" @click="doLogout" style="margin-left: 12px;">退出登录</el-button>
        </div>

        <!-- 演员管理 -->
        <h3>演员文件夹管理</h3>
        <el-form label-width="160px" size="small">
          <el-form-item label="演员文件夹路径">
            <el-input v-model="actorBaseDir" style="width: 360px;" />
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
          <el-form-item label="合并文件夹">
            <el-switch v-model="mergeAutoFolders" />
          </el-form-item>
          <el-form-item label="合并文件命名">
            <el-input v-model="mergeFilePattern" style="width: 200px;" />
          </el-form-item>
          <el-form-item label="合并默认模拟运行">
            <el-switch v-model="mergeDryRun" />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="saveActorSettings">保存演员设置</el-button>
          </el-form-item>
        </el-form>

        <!-- 刮削引擎更新 -->
        <h3 style="margin-top:24px;">刮削引擎</h3>
        <div style="margin-bottom:12px; font-size:12px; color:#9090a0;">
          MetaTube SDK — 社区维护的39个刮削提供器，点击按钮自动拉取最新代码并编译
        </div>
        <el-button type="warning" @click="updateMetaTube" :loading="metaTubeLoading">
          {{ metaTubeLoading ? '更新中...' : '更新刮削引擎' }}
        </el-button>
        <p v-if="metaTubeResult" style="margin-top:8px; font-size:12px; color:#67c23a; white-space:pre-wrap;">{{ metaTubeResult }}</p>
        <p v-if="metaTubeError" style="margin-top:8px; font-size:12px; color:#f56c6c;">{{ metaTubeError }}</p>
      </el-tab-pane>

      <!-- 标签库 -->
      <el-tab-pane label="标签库" name="genres">
        <div style="margin-bottom:8px; font-size:12px; color:#9090a0;">日文标签 → 中文翻译（刮削时自动填充），双击中文列可编辑</div>
        <el-table :data="genreLib" size="small" max-height="500" style="width:100%;" @cell-dblclick="editGenreCell">
          <el-table-column prop="ja_name" label="日文" width="220" />
          <el-table-column label="中文" min-width="220">
            <template #default="{ row, $index }">
              <el-input v-if="editingGenre === $index" v-model="row.cn_name" size="small" @blur="saveGenreRow(row)" @keyup.enter="saveGenreRow(row)" />
              <span v-else style="cursor:pointer;">{{ row.cn_name }}</span>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ row }">
              <el-button size="small" text type="danger" @click="deleteGenreRow(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
        <div style="margin-top:8px; display:flex; gap:8px;">
          <el-input v-model="newGenreJa" size="small" placeholder="日文标签" style="width:160px;" @keyup.enter="addGenreRow" />
          <el-input v-model="newGenreCn" size="small" placeholder="中文翻译" style="width:160px;" @keyup.enter="addGenreRow" />
          <el-button size="small" @click="addGenreRow">添加</el-button>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'

const activeTab = ref('scrape')

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

// Scrape
const scrapeSources = ref<string[]>([])
const videoExts = ref('')
const tmdbApiKey = ref('')
const jphooSecret = ref('')
const jphooRefresh = ref('')
const jphooGuestId = ref('')
const deepseekApiKey = ref('')

const sourceTable = reactive([
  { key: 'tmdb', name: 'TMDB', site: 'api.themoviedb.org', type: '通用', done: true, remark: '', defaultRemark: '' },
  { key: 'imdb', name: 'IMDb', site: 'www.imdb.com', type: '通用', done: true, remark: '', defaultRemark: '' },
  { key: 'douban', name: '豆瓣', site: 'movie.douban.com', type: '通用', done: true, remark: '', defaultRemark: '' },
  { key: 'javbus', name: 'JavBus', site: 'www.javbus.com', type: 'AV', done: true, remark: '', defaultRemark: '需代理访问' },
  { key: 'javdb', name: 'JavDB', site: 'javdb.com', type: 'AV', done: true, remark: '', defaultRemark: '需代理访问' },
  { key: 'javlibrary', name: 'JavLibrary', site: 'www.javlibrary.com', type: 'AV', done: true, remark: '', defaultRemark: '需代理访问' },
  { key: 'fanza', name: 'Fanza', site: 'www.dmm.co.jp', type: 'AV', done: true, remark: '', defaultRemark: '需代理访问' },
  { key: 'arzon', name: 'Arzon', site: 'www.arzon.jp', type: 'AV', done: true, remark: '', defaultRemark: '' },
  { key: 'mgstage', name: 'MGStage', site: 'www.mgstage.com', type: 'AV', done: true, remark: '', defaultRemark: '' },
  { key: 'fc2', name: 'FC2', site: 'adult.contents.fc2.com', type: 'AV', done: true, remark: '', defaultRemark: '' },
  { key: 'airav', name: 'Airav', site: 'www.airav.wiki', type: 'AV', done: true, remark: '', defaultRemark: '可能需代理' },
  { key: 'xcity', name: 'XCITY', site: 'www.xcity.jp', type: 'AV', done: true, remark: '', defaultRemark: '需代理访问' },
  { key: 'jav321', name: 'Jav321', site: 'www.jav321.com', type: 'AV', done: true, remark: '', defaultRemark: '站点不稳定' },
  { key: 'prestige', name: 'Prestige', site: 'www.prestige-av.com', type: 'AV', done: true, remark: '', defaultRemark: '需代理访问' },
  { key: 'avsox', name: 'Avsox', site: 'avsox.cyou', type: 'AV', done: true, remark: '', defaultRemark: '可能需代理' },
  { key: 'njav', name: 'Njav', site: 'njav.tv', type: 'AV', done: true, remark: '', defaultRemark: '可能需代理' },
  { key: 'getav', name: 'GetAV', site: 'getav.info', type: 'AV', done: true, remark: '', defaultRemark: '可能需代理' },
  { key: 'whostv', name: 'WhosTV', site: 'whostv.net', type: 'AV', done: true, remark: '', defaultRemark: '可能需代理' },
  { key: 'jphoo', name: 'JpHoo', site: 'www.jphoo1.com', type: 'AV', done: true, remark: '', defaultRemark: '需配置secret/refreshtoken/guestid' },
  { key: 'fc2ppvdb', name: 'FC2PPVDB', site: 'fc2ppvdb.com', type: 'AV', done: true, remark: '', defaultRemark: '需代理访问' },
  { key: 'metatube', name: 'MetaTube', site: '39个社区提供器', type: '聚合', done: true, remark: '', defaultRemark: '需在常规页启动MetaTube服务' },
  { key: 'deepseek', name: 'DeepSeek', site: 'api.deepseek.com', type: '翻译', done: true, remark: '', defaultRemark: '用于翻译片名，需API Key' },
])

const allChecked = computed(() => sourceTable.every(r => scrapeSources.value.includes(r.key)))
const indeterminate = computed(() => !allChecked.value && sourceTable.some(r => scrapeSources.value.includes(r.key)))
function toggleAllSources(v: boolean) {
  scrapeSources.value = v ? sourceTable.map(r => r.key) : []
}

function toggleSource(key: string, enabled: boolean) {
  if (enabled) { if (!scrapeSources.value.includes(key)) scrapeSources.value.push(key) }
  else { scrapeSources.value = scrapeSources.value.filter(s => s !== key) }
}

// Proxy
const proxyEnabled = ref(false)
const proxyType = ref('http')
const proxyHost = ref('127.0.0.1')
const proxyPort = ref(1080)

// Cache
const cacheMaxSize = ref(2048)

// UI
const theme = ref('dark')
const posterSize = ref('medium')
const fontSize = ref(14)
const privacyTitle = ref('智能网盘影视库')
const externalPlayer = ref('')
const refreshInterval = ref(240)
const autoStart = ref(false)

// Actor
const actorBaseDir = ref('')
const autoCreateActors = ref(true)
const actorPendingReview = ref(true)
const allowRenameFolder = ref(false)
const mergeAutoFolders = ref(true)
const mergeFilePattern = ref('{name}_{index}{ext}')
const mergeDryRun = ref(true)

// MetaTube SDK update
const metaTubeLoading = ref(false)
const metaTubeResult = ref('')
const metaTubeError = ref('')
async function updateMetaTube() {
  metaTubeLoading.value = true; metaTubeResult.value = ''; metaTubeError.value = ''
  try {
    metaTubeResult.value = await invoke('update_metatube_sdk') as string
  } catch (e: any) {
    metaTubeError.value = String(e?.message || e)
  } finally { metaTubeLoading.value = false }
}

// Genre translation library
interface GenreRow { ja_name: string; cn_name: string }
const genreLib = ref<GenreRow[]>([])
const newGenreJa = ref('')
const newGenreCn = ref('')
const editingGenre = ref<number | null>(null)

async function loadGenreLib() {
  try { genreLib.value = await invoke('get_genre_translations') as any[] || [] }
  catch { genreLib.value = [] }
}
function editGenreCell(row: any, _col: any, _cell: any, _event: any) {
  editingGenre.value = genreLib.value.indexOf(row)
}
async function saveGenreRow(row: GenreRow) {
  editingGenre.value = null
  if (row.ja_name && row.cn_name) {
    await invoke('set_genre_translation', { jaName: row.ja_name, cnName: row.cn_name })
  }
}
async function deleteGenreRow(row: GenreRow) {
  await invoke('set_genre_translation', { jaName: row.ja_name, cnName: '' })
  genreLib.value = genreLib.value.filter(r => r.ja_name !== row.ja_name)
}
async function addGenreRow() {
  if (newGenreJa.value && newGenreCn.value) {
    await invoke('set_genre_translation', { jaName: newGenreJa.value, cnName: newGenreCn.value })
    genreLib.value.push({ ja_name: newGenreJa.value, cn_name: newGenreCn.value })
    newGenreJa.value = ''; newGenreCn.value = ''
  }
}

// ─── QR Code Login ───

async function startLogin() {
  loginLoading.value = true; qrError.value = ''; loginStatusText.value = '获取二维码中...'; pollCount = 0
  try {
    const result: any = await invoke('login_qrcode')
    qrcodeUrl.value = result.qrcode_url; loginUid.value = result.uid
    loginStatusText.value = '等待扫码 (轮询中...)'
    loginTimer = setInterval(checkLoginStatus, 2000)
  } catch (e: any) {
    qrError.value = '获取二维码失败: ' + (e.message || e)
    ElMessage.error(qrError.value)
    loginLoading.value = false
  }
}

let pollCount = 0
async function checkLoginStatus() {
  pollCount++
  try {
    const result: any = await invoke('login_status', { uid: loginUid.value })
    const statusMap: Record<string, string> = { waiting: '等待扫码', scanned: '已扫码，请确认', authorized: '已授权，登录中...', expired: '已过期' }
    loginStatusText.value = statusMap[result.status] || result.status
    loginStatusText.value += ` (${pollCount})`
    if (result.status === 'authorized') {
      if (result.cookie) {
        await invoke('login_cookie', { cookie: result.cookie })
        loggedIn.value = true; loginLoading.value = false
        if (loginTimer) clearInterval(loginTimer)
        ElMessage.success('登录成功！')
      } else {
        loginStatusText.value = '已授权，等待Cookie... (' + pollCount + ')'
      }
    } else if (result.status === 'expired') {
      loginLoading.value = false
      if (loginTimer) clearInterval(loginTimer)
      ElMessage.warning('二维码已过期，请重新获取')
    }
  } catch (e: any) {
    loginStatusText.value = `轮询异常(${pollCount}): ` + (e.message || e)
  }
}

async function loginByCookie() {
  if (!cookieInput.value.trim()) { ElMessage.warning('请先粘贴Cookie'); return }
  cookieLoading.value = true
  try {
    await invoke('login_cookie_direct', { cookie: cookieInput.value.trim() })
    loggedIn.value = true
    ElMessage.success('Cookie登录成功')
  } catch (e: any) {
    const msg = (e?.message || e || '') + '\n日志路径: %APPDATA%\\smart-media-vault\\logs\\app.log'
    ElMessage({ message: 'Cookie验证失败: ' + msg, type: 'error', duration: 10000, showClose: true })
  } finally { cookieLoading.value = false }
}

async function doLogout() {
  await invoke('logout')
  loggedIn.value = false
  ElMessage.success('已退出登录')
}

// ─── Save Handlers ───

async function saveScrapeSettings() {
  await invoke('set_config', { key: 'scrape_sources', value: JSON.stringify(scrapeSources.value) })
  // Save remarks (user-editable)
  const remarks: Record<string, string> = {}
  sourceTable.forEach(r => { if (r.remark) remarks[r.key] = r.remark })
  await invoke('set_config', { key: 'scrape_remarks', value: JSON.stringify(remarks) })
  if (tmdbApiKey.value) await invoke('set_secure_config', { key: 'tmdb_api_key', value: tmdbApiKey.value })
  if (deepseekApiKey.value) await invoke('set_secure_config', { key: 'deepseek_api_key', value: deepseekApiKey.value })
  await invoke('set_config', { key: 'jphoo_secret', value: jphooSecret.value.trim() })
  await invoke('set_config', { key: 'jphoo_refreshtoken', value: jphooRefresh.value.trim() })
  await invoke('set_config', { key: 'jphoo_guestid', value: jphooGuestId.value.trim() })
  // 验证保存结果
  const savedSecret = await invoke('get_config', { key: 'jphoo_secret' })
  const savedRefresh = await invoke('get_config', { key: 'jphoo_refreshtoken' })
  console.log('JpHoo配置已保存: secret=' + (savedSecret ? (savedSecret as string).length + '字节' : '空') + ' refresh=' + (savedRefresh ? (savedRefresh as string).length + '字节' : '空'))
  ElMessage.success('刮削设置已保存')
}

async function saveProxySettings() {
  await invoke('set_config', { key: 'proxy_enabled', value: String(proxyEnabled.value) })
  await invoke('set_config', { key: 'proxy_host', value: proxyHost.value })
  await invoke('set_config', { key: 'proxy_port', value: String(proxyPort.value) })
  ElMessage.success('代理设置已保存')
}

async function saveCacheSettings() {
  await invoke('set_config', { key: 'cache_max_size', value: String(cacheMaxSize.value * 1024 * 1024) })
  ElMessage.success('缓存设置已保存')
}

async function saveUiSettings() {
  await invoke('set_config', { key: 'theme', value: theme.value })
  await invoke('set_config', { key: 'poster_size', value: posterSize.value })
  await invoke('set_config', { key: 'font_size', value: String(fontSize.value) })
  await invoke('set_config', { key: 'privacy_title', value: privacyTitle.value })
  await invoke('set_config', { key: 'external_player', value: externalPlayer.value })
  await invoke('set_config', { key: 'playback_refresh_interval', value: String(refreshInterval.value) })
  await invoke('set_config', { key: 'auto_start', value: String(autoStart.value) })
  ElMessage.success('界面设置已保存')
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

async function exportList() {
  const path = await invoke('export_list')
  ElMessage.success('已导出到: ' + path)
}

async function cleanGroups() {
  try {
    await ElMessageBox.confirm('确定清理所有分组数据？（不会影响影片和演员）', '确认', { type: 'warning' })
    const result = await invoke('clean_all_groups')
    ElMessage.success(String(result))
    // Refresh sidebar groups
    window.dispatchEvent(new CustomEvent('groups-changed'))
  } catch (e: any) { if (e !== 'cancel') ElMessage.error(String(e?.message || e)) }
}

async function clearCache() {
  ElMessage.info('缓存清理功能待实现')
}

onMounted(async () => {
  try {
    loggedIn.value = await invoke('check_token')
    const sources: string | null = await invoke('get_config', { key: 'scrape_sources' })
    if (sources) scrapeSources.value = JSON.parse(sources)
    const exts: string | null = await invoke('get_config', { key: 'video_extensions' })
    if (exts) { try { videoExts.value = JSON.parse(exts).join(',') } catch { videoExts.value = exts } }
    theme.value = (await invoke('get_config', { key: 'theme' }) as string) || 'dark'
    posterSize.value = (await invoke('get_config', { key: 'poster_size' }) as string) || 'medium'
    const fz: string | null = await invoke('get_config', { key: 'font_size' })
    if (fz) fontSize.value = parseInt(fz)
    privacyTitle.value = (await invoke('get_config', { key: 'privacy_title' }) as string) || '智能网盘影视库'
    externalPlayer.value = (await invoke('get_config', { key: 'external_player' }) as string) || ''
    const interval: string | null = await invoke('get_config', { key: 'playback_refresh_interval' })
    if (interval) refreshInterval.value = parseInt(interval)
    autoStart.value = (await invoke('get_config', { key: 'auto_start' }) as string) === 'true'

    const proxy: string | null = await invoke('get_config', { key: 'proxy_enabled' })
    proxyEnabled.value = proxy === 'true'
    const ph: string | null = await invoke('get_config', { key: 'proxy_host' })
    if (ph) proxyHost.value = ph
    const pp: string | null = await invoke('get_config', { key: 'proxy_port' })
    if (pp) proxyPort.value = parseInt(pp) || 1080

    const cm: string | null = await invoke('get_config', { key: 'cache_max_size' })
    if (cm) cacheMaxSize.value = Math.round(parseInt(cm) / 1024 / 1024)

    actorBaseDir.value = (await invoke('get_config', { key: 'local_actor_base_dir' }) as string) || ''
    autoCreateActors.value = ((await invoke('get_config', { key: 'auto_create_actors_from_scrape' }) as string) || '1') === '1'
    actorPendingReview.value = ((await invoke('get_config', { key: 'actor_pending_review' }) as string) || '1') === '1'
    allowRenameFolder.value = ((await invoke('get_config', { key: 'allow_app_rename_actor_folders' }) as string) || '0') === '1'
    mergeAutoFolders.value = ((await invoke('get_config', { key: 'actor_merge_auto_merge_folders' }) as string) || '1') === '1'
    mergeFilePattern.value = (await invoke('get_config', { key: 'actor_merge_file_naming_pattern' }) as string) || '{name}_{index}{ext}'
    mergeDryRun.value = ((await invoke('get_config', { key: 'actor_merge_dry_run' }) as string) || '1') === '1'

    tmdbApiKey.value = (await invoke('get_secure_config', { key: 'tmdb_api_key' }) as string) || ''
    deepseekApiKey.value = (await invoke('get_secure_config', { key: 'deepseek_api_key' }) as string) || ''
    jphooSecret.value = (await invoke('get_config', { key: 'jphoo_secret' }) as string) || ''
    jphooRefresh.value = (await invoke('get_config', { key: 'jphoo_refreshtoken' }) as string) || ''
    jphooGuestId.value = (await invoke('get_config', { key: 'jphoo_guestid' }) as string) || ''
    loadGenreLib()
    const remarks = (await invoke('get_config', { key: 'scrape_remarks' }) as string) || '{}'
    try {
      const rm: Record<string, string> = JSON.parse(remarks)
      sourceTable.forEach(r => { if (rm[r.key]) r.remark = rm[r.key] })
    } catch { /* ignore */ }
  } catch { /* config may not exist yet */ }
})
</script>

<style scoped>
.settings-page { width: 100%; }
h2 { font-size: 20px; margin-bottom: 16px; }
h3 { font-size: 15px; margin-bottom: 12px; color: #e0e0e0; }
.danger-zone { }
.danger-zone h4 { color: #f56c6c; font-size: 14px; margin-bottom: 12px; }

/* Tab 对比度修复 */
:deep(.el-tabs__item) {
  color: #9090a0;
  font-weight: 500;
}
:deep(.el-tabs__item.is-active) {
  color: #409eff;
  font-weight: 600;
}
:deep(.el-tabs__item:hover) {
  color: #c0c0d0;
}
:deep(.el-tabs--border-card) {
  background: transparent;
  border-color: #2a2a4a;
}
:deep(.el-tabs--border-card > .el-tabs__header) {
  background: #1a1a2e;
  border-bottom-color: #2a2a4a;
}
:deep(.el-tabs--border-card > .el-tabs__header .el-tabs__item.is-active) {
  background: #252540;
  border-color: #2a2a4a;
  color: #409eff;
}
:deep(.el-tabs--border-card > .el-tabs__content) {
  padding: 16px;
}
</style>
