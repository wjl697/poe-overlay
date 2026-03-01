import { createRouter, createWebHistory } from 'vue-router'
import MainOverlay from './views/MainOverlay.vue'
import ActionBar from './views/ActionBar.vue'

const router = createRouter({
    history: createWebHistory(),
    routes: [
        {
            path: '/',
            name: 'home',
            component: MainOverlay
        },
        {
            path: '/actionbar',
            name: 'actionbar',
            component: ActionBar
        }
    ]
})

export default router
