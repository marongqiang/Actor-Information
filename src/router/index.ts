import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/views/PosterWall.vue'),
    },
    {
      path: '/detail/:fileId',
      name: 'detail',
      component: () => import('@/views/Detail.vue'),
    },
    {
      path: '/player/:fileId',
      name: 'player',
      component: () => import('@/views/Player.vue'),
    },
    {
      path: '/favorites',
      name: 'favorites',
      component: () => import('@/views/Favorites.vue'),
    },
    {
      path: '/actress',
      name: 'actress',
      component: () => import('@/views/Actress.vue'),
    },
    {
      path: '/actress-table',
      name: 'actress-table',
      component: () => import('@/views/ActressTable.vue'),
    },
    {
      path: '/scan',
      name: 'scan',
      component: () => import('@/views/Scan.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/Settings.vue'),
    },
  ],
})

// 路由守卫：不再强制要求登录，115相关操作会在调用API时提示未登录
router.beforeEach(async (_to, _from, next) => {
  next()
})

export default router
