import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/views/PosterWall.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/detail/:fileId',
      name: 'detail',
      component: () => import('@/views/Detail.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/player/:fileId',
      name: 'player',
      component: () => import('@/views/Player.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/actress',
      name: 'actress',
      component: () => import('@/views/Actress.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/actress-table',
      name: 'actress-table',
      component: () => import('@/views/ActressTable.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/scan',
      name: 'scan',
      component: () => import('@/views/Scan.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/Settings.vue'),
      meta: { requiresAuth: false },
    },
  ],
})

router.beforeEach(async (to, _from, next) => {
  const requiresAuth = to.meta.requiresAuth !== false
  if (requiresAuth) {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const isLoggedIn = await invoke('check_token')
      if (!isLoggedIn) {
        next('/settings')
        return
      }
    } catch {
      next('/settings')
      return
    }
  }
  next()
})

export default router
