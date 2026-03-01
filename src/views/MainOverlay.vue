<script setup lang="ts">
import { useOverlayStore } from '../store/overlay';

const store = useOverlayStore();
const startDrag = () => invoke('start_drag');

// 计算剧情区动态偏移，模拟苹果滚轮特效
const getStepClass = (index: number) => {
  const diff = index - store.currentStepIndex;
  if (diff === 0) return 'story-focus-center';
  if (Math.abs(diff) === 1) return 'story-focus-adjacent';
  return 'story-focus-edge';
};

import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { onMounted, onUnmounted, onBeforeUpdate, ref, watch, nextTick } from 'vue';

const isDragOver = ref(false);
const showDonate = ref(false);
const showChapterMenu = ref(false);

// 动态计算中心焦点的滚动位移
const scrollOffsetY = ref(0);
const scrollContainerRef = ref<HTMLElement | null>(null);
const stepRefs = ref<HTMLElement[]>([]);

onBeforeUpdate(() => {
  stepRefs.value = [];
});

const updateScrollOffset = () => {
  if (!scrollContainerRef.value || stepRefs.value.length === 0) return;
  const container = scrollContainerRef.value.parentElement;
  if (!container) return;

  const activeIndex = store.currentStepIndex;
  const activeEl = stepRefs.value[activeIndex];
  if (!activeEl) return;

  // 容器的中心点
  const containerCenter = container.clientHeight / 2;
  // 当前元素的中心点（相对于滚动容器内部顶部的距离）
  const elCenter = activeEl.offsetTop + activeEl.offsetHeight / 2;

  // 最终 translateY 位移量
  scrollOffsetY.value = elCenter - containerCenter;
};

// 监听状态变化：数据加载完毕，或索引切换时，重新计算位移
watch([() => store.currentStepIndex, () => store.steps, () => store.isMinimal], async () => {
  await nextTick();
  updateScrollOffset();
});

// 文件对话框选择文件
const loadDocViaDialog = async () => {
  try {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: '文档', extensions: ['md', 'txt'] }],
    });
    if (selected && typeof selected === 'string') {
      await invoke('start_file_watcher', { path: selected });
      await store.setDocumentPath(selected);
    }
  } catch (err) {
    console.error('Failed to open file dialog:', err);
  }
};

// DOM 层仅用于触发视觉反馈（isDragOver 高亮）
// 实际文件路径由 Tauri 原生 onDragDropEvent 提供（见 onMounted）
const onDomDragOver = (event: DragEvent) => {
  event.preventDefault();
  isDragOver.value = true;
};

const onDomDragLeave = (event: DragEvent) => {
  // 仅当鼠标真正离开整个窗口时才重置
  if (!event.relatedTarget) isDragOver.value = false;
};

const onDomDrop = (event: DragEvent) => {
  // 阻止浏览器默认打开文件行为，路径由 Tauri 原生事件处理
  event.preventDefault();
};

// 窗口控制
const minimizeWindow = () => invoke('minimize_window');
const closeWindow = () => invoke('close_window');

const donateRef = ref<HTMLElement | null>(null);
const chapterMenuRef = ref<HTMLElement | null>(null);

// 点击外部关闭弹窗
const onDocClick = (e: MouseEvent) => {
  const target = e.target as Node;
  if (showDonate.value && donateRef.value && !donateRef.value.contains(target)) {
    showDonate.value = false;
  }
  if (showChapterMenu.value && chapterMenuRef.value && !chapterMenuRef.value.contains(target)) {
    showChapterMenu.value = false;
  }
};

// 保存所有 unlisten 函数，在组件卸载时清理，防止 HMR 时监听器累积
let unlistenPrev: (() => void) | null = null;
let unlistenNext: (() => void) | null = null;
let unlistenParsed: (() => void) | null = null;
let unlistenDragDrop: (() => void) | null = null;

onUnmounted(() => {
  document.removeEventListener('click', onDocClick, true);
  unlistenPrev?.();
  unlistenNext?.();
  unlistenParsed?.();
  unlistenDragDrop?.();
});

onMounted(async () => {
  // 加载持久化状态
  await store.initializeStore();

  document.addEventListener('click', onDocClick, true);
  unlistenPrev = await listen('action-bar-prev', () => store.prevStep());
  unlistenNext = await listen('action-bar-next', () => store.nextStep());

  unlistenParsed = await listen<{notes: string, steps: {id: string, chapter: string, text: string}[]}>('parsed-document', (event) => {
    store.updateParsedDocument(event.payload);
  });

  // 使用 Tauri 原生拖拽事件，可靠获取本地文件路径
  const win = getCurrentWindow();
  unlistenDragDrop = await win.onDragDropEvent(async (event) => {
    if (event.payload.type === 'enter') {
      isDragOver.value = true;
    } else if (event.payload.type === 'leave') {
      isDragOver.value = false;
    } else if (event.payload.type === 'drop') {
      isDragOver.value = false;
      const paths: string[] = (event.payload.paths as string[]) ?? [];
      const filePath = paths.find(p => p.endsWith('.md') || p.endsWith('.txt'));
      if (filePath) {
        await invoke('start_file_watcher', { path: filePath });
        await store.setDocumentPath(filePath);
      }
    }
  });

  // 初始计算一次高度
  await nextTick();
  updateScrollOffset();
});
</script>

<template>
  <!-- 最外层容器，支持拖拽文件进入 -->
  <div 
    class="w-full h-full flex flex-col relative transition-colors duration-300"
    :class="[
      store.isMinimal ? 'bg-transparent border-0 shadow-none ring-0' : 'bg-black/75',
      isDragOver ? 'ring-2 ring-blue-400/80 ring-inset' : ''
    ]"
    :style="{ fontSize: store.fontSize + 'px' }"
    @drop="onDomDrop"
    @dragover="onDomDragOver"
    @dragleave="onDomDragLeave"
  >
    <!-- 拖拽遮罩层提示 -->
    <Transition name="fade">
      <div 
        v-if="isDragOver"
        class="absolute inset-0 z-50 flex items-center justify-center pointer-events-none"
        style="background: rgba(59,130,246,0.15);"
      >
        <div class="text-blue-300 text-base font-bold tracking-wider drop-shadow-lg flex flex-col items-center gap-2">
          <span class="text-2xl">📄</span>
          <span>松开以加载文档</span>
        </div>
      </div>
    </Transition>

    <!-- Top Hot Zone (48px) 防死锁唤醒区 -->
    <div 
      class="absolute top-0 left-0 w-full h-[48px] z-30 peer"
      @mousedown="startDrag"
    ></div>

    <!-- 1. 顶部设置区 (Top Bar) -->
    <div 
      class="w-full h-[48px] flex-shrink-0 flex items-center justify-between px-3 bg-gray-900/90 border-b border-gray-700/50 transition-transform duration-300 z-40 relative"
      :class="[
        store.isMinimal ? '-translate-y-full peer-hover:translate-y-0 hover:translate-y-0 opacity-0 peer-hover:opacity-100 hover:opacity-100' : 'translate-y-0 opacity-100'
      ]"
      @mousedown="startDrag"
    >
      <div class="text-gray-300 font-bold text-sm tracking-wider">POE</div>
      
      <!-- 控制按钮区 -->
      <div class="flex items-center gap-1" @mousedown.stop>
        <!-- 章节目录 -->
        <div ref="chapterMenuRef" class="relative" @mousedown.stop>
          <button 
            @click="showChapterMenu = !showChapterMenu" 
            class="icon-btn" 
            :class="store.chapterList.length > 0 ? 'text-blue-400 hover:text-blue-300' : 'text-gray-600 cursor-not-allowed'"
            :disabled="store.chapterList.length === 0"
            data-tip="章节跳转" 
            data-pos="left"
          >
            📘
          </button>
          <!-- 章节目录弹窗菜单 -->
          <Transition name="fade">
            <div 
              v-if="showChapterMenu" 
              class="absolute top-full left-1/2 -translate-x-1/2 mt-2 py-2 w-40 max-h-[250px] overflow-y-auto bg-gray-900 border border-poe-gold/30 rounded shadow-xl z-50 custom-scrollbar"
            >
              <div 
                v-for="chapter in store.chapterList" 
                :key="chapter.startIndex"
                class="px-4 py-2 text-sm text-gray-300 hover:bg-gray-800 hover:text-white cursor-pointer transition-colors"
                @click="() => { store.jumpToStep(chapter.startIndex); showChapterMenu = false; }"
              >
                {{ chapter.name }}
              </div>
            </div>
          </Transition>
        </div>
        <div class="sep"></div>

        <!-- 加载文档 -->
        <button @click="loadDocViaDialog" class="icon-btn text-blue-400" data-tip="加载文档" data-pos="bottom">
          📂
        </button>
        <div class="sep"></div>
        <!-- 缩小字体 -->
        <button @click="store.zoomOut" class="icon-btn" data-tip="缩小字体">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <circle cx="6.5" cy="6.5" r="5" stroke="currentColor" stroke-width="1.5" fill="none"/>
            <line x1="4" y1="6.5" x2="9" y2="6.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <line x1="10.5" y1="10.5" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
        <!-- 放大字体 -->
        <button @click="store.zoomIn" class="icon-btn" data-tip="放大字体">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <circle cx="6.5" cy="6.5" r="5" stroke="currentColor" stroke-width="1.5" fill="none"/>
            <line x1="4" y1="6.5" x2="9" y2="6.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <line x1="6.5" y1="4" x2="6.5" y2="9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <line x1="10.5" y1="10.5" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
        <!-- 极简/完整模式 -->
        <button
          @click="store.toggleMinimal"
          class="icon-btn"
          :class="store.isMinimal ? 'text-poe-gold' : 'text-gray-400'"
          :data-tip="store.isMinimal ? '切换到完整模式' : '切换到极简模式'"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
            <rect x="1.5" y="1.5" width="13" height="13" rx="2" stroke="currentColor" stroke-width="1.5"/>
            <line x1="1.5" y1="5" x2="14.5" y2="5" stroke="currentColor" stroke-width="1.2"/>
            <circle v-if="store.isMinimal" cx="8" cy="10" r="2" fill="currentColor"/>
          </svg>
        </button>
        <!-- 鼠标穿透 -->
        <button
          @click="store.togglePassthrough"
          class="icon-btn"
          :class="store.isPassthrough ? 'text-red-400' : 'text-gray-400'"
          :data-tip="store.isPassthrough ? '关闭鼠标穿透' : '开启鼠标穿透'"
          data-pos="right"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
            <path d="M4 2 L4 11 L6.5 8.5 L8 12 L9.5 11.5 L8 8 L11 8 Z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" fill="none"/>
            <line v-if="store.isPassthrough" x1="2" y1="2" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
        <div class="sep"></div>
        <!-- 打赏按钮 -->
        <div ref="donateRef" class="relative" @mousedown.stop>
          <button
            @click="showDonate = !showDonate"
            class="icon-btn text-red-400 hover:text-red-300"
            data-tip="请喝可乐"
            data-pos="left"
          >
            🥤
          </button>
          <!-- 气泡卡片：点击图标弹出 -->
          <Transition name="bubble">
            <div
              v-if="showDonate"
              class="absolute right-0 top-full mt-2 z-50 bg-gray-900 border border-gray-600 rounded-xl shadow-2xl p-3 flex flex-col items-center gap-2"
              style="width: 170px;"
            >
              <!-- 上箭头 -->
              <div class="absolute -top-[7px] right-2 w-3 h-3 bg-gray-900 border-t border-l border-gray-600 rotate-45"></div>
              <p class="text-yellow-300 text-xs font-semibold tracking-wide">🥤 请喝可乐</p>
              <img
                src="../assets/donate_qr.png"
                alt="收款码"
                class="w-full rounded-lg object-contain"
                style="max-height: 150px;"
              />
              <!-- B站图标 + 感谢文字同一行 -->
              <div class="flex items-center gap-1.5">
                <button
                  @click="invoke('open_url', { url: 'https://space.bilibili.com/99815426' })"
                  class="icon-btn active:scale-90 flex-shrink-0"
                  style="color: #00a1d6;"
                  data-tip="关注作者 B站"
                  data-pos="right"
                >
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.658.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773H5.333zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.249-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c0-.373.129-.689.386-.947.258-.257.574-.386.947-.386zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.249-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"/>
                  </svg>
                </button>
                <p class="text-gray-500 text-[10px]">支持即鼓励，感谢！</p>
              </div>
            </div>
          </Transition>
        </div>
        <div class="sep"></div>
        <!-- 最小化 -->
        <button @click="minimizeWindow" class="win-btn hover:text-yellow-300 hover:bg-yellow-500/10">
          &#x2013;
        </button>
        <!-- 关闭 -->
        <button @click="closeWindow" class="win-btn hover:text-red-400 hover:bg-red-500/10">
          &#x2715;
        </button>
      </div>
    </div>

    <!-- 主体内容容器 -->
    <div 
      class="flex flex-col flex-grow overflow-hidden"
      :class="store.isMinimal ? '-translate-y-[48px]' : 'translate-y-0'"
      style="transition: transform 300ms ease; will-change: transform;"
    >
      <!-- 2. 中部备注区 -->
      <div 
        class="w-full p-3 text-gray-300 flex-shrink-0 transition-colors duration-300"
        :class="store.isMinimal ? 'border-transparent bg-transparent tracking-wide' : 'bg-black/60 border-b border-gray-700/50'"
        :style="{ maxHeight: `${store.maxNotesLines * 1.5}em`, overflowY: store.isPassthrough ? 'hidden' : 'auto' }"
      >
        <div v-if="store.notes === '等待解析文档...'" class="text-gray-500 italic text-xs flex items-center gap-2">
          <span>点击 📂 「加载文档」或将 .md/.txt 文件拖入窗口</span>
        </div>
        <div 
          v-else 
          class="whitespace-pre-wrap leading-relaxed"
          :class="store.isPassthrough ? 'select-none pointer-events-none' : 'select-text cursor-text'"
        >
          {{ store.notes }}
        </div>
      </div>

      <!-- 3. 底部剧情区 -->
      <div 
        class="flex-grow relative overflow-hidden flex flex-col justify-center px-4 transition-colors duration-300 bg-transparent"
      >
        <!-- 内部滚动容器：通过 JS 动态计算的 translateY 实现精确对中 -->
        <div 
          ref="scrollContainerRef"
          class="flex flex-col transform transition-transform duration-[250ms] ease-out absolute w-full top-0 left-0 px-4 py-[50vh]"
          :style="{ transform: `translateY(-${scrollOffsetY}px)` }"
        >
          <div 
            v-for="(step, index) in store.steps" 
            :key="step.id"
            :ref="(el) => { if (el) stepRefs[index] = el as HTMLElement }"
            class="flex items-center w-full transform-origin-left my-2 transition-all duration-[250ms]"
            :class="[
              getStepClass(index),
              index === store.currentStepIndex 
                ? 'py-3 pr-10 whitespace-normal break-words leading-snug' 
                : 'h-[3rem] truncate'
            ]"
          >
            <span>{{ step.text }}</span>
          </div>
        </div>
      </div>
    </div>

  </div>
</template>

<style scoped>
@reference "../style.css";

/* 图标按钮：统一尺寸，CSS tooltip */
.icon-btn {
  @apply w-7 h-7 flex items-center justify-center rounded text-gray-400 transition-all cursor-pointer;
  position: relative;
  font-size: 15px;
}
.icon-btn:hover {
  @apply bg-gray-700/60 text-gray-100;
}
/* 自定义 tooltip via data-tip */
.icon-btn::after {
  content: attr(data-tip);
  position: absolute;
  top: 100%;
  margin-top: 6px;
  background: rgba(15, 15, 25, 0.96);
  color: #cbd5e1;
  font-size: 11px;
  white-space: nowrap;
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid rgba(255,255,255,0.08);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s ease;
  z-index: 999;
  
  /* 默认居中 */
  left: 50%;
  transform: translateX(-50%);
}

/* 根据 data-pos 调整对齐点 */
.icon-btn[data-pos="left"]::after {
  left: 0;
  transform: translateX(0);
}
.icon-btn[data-pos="right"]::after {
  left: auto;
  right: 0;
  transform: translateX(0);
}

.icon-btn:hover::after {
  opacity: 1;
}

/* 分隔线 */
.sep {
  @apply w-[1px] h-4 bg-gray-700 mx-0.5;
}

/* 最小化/关闭：无 tooltip，稍小尺寸 */
.win-btn {
  @apply w-6 h-6 flex items-center justify-center rounded text-gray-500 transition-all cursor-pointer;
  font-size: 14px;
  line-height: 1;
}

.transform-origin-left {
  transform-origin: left center;
}

/* 拖拽遮罩淡入淡出 */
.fade-enter-active, .fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}

/* 打赏气泡弹出动画 */
.bubble-enter-active {
  transition: opacity 0.15s ease, transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.bubble-leave-active {
  transition: opacity 0.1s ease, transform 0.1s ease;
}
.bubble-enter-from, .bubble-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.95);
}

/* 核心滚轮焦点状态 */
.story-focus-center {
  @apply opacity-100 font-semibold text-text-pure-white;
  transform: scale(1.15);
  text-shadow: 0 0 8px rgba(255, 255, 255, 0.4);
  transition: all 250ms cubic-bezier(0.25, 0.8, 0.25, 1);
}

.story-focus-adjacent {
  @apply opacity-60 text-gray-400;
  transform: scale(1.0);
  transition: all 250ms ease;
}

.story-focus-edge {
  @apply opacity-20 text-gray-500;
  transform: scale(0.9);
  transition: all 250ms ease;
}
</style>
