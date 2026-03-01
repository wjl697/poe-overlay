<script setup lang="ts">
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { ref, onMounted } from 'vue';

const isPassthrough = ref(false);

onMounted(async () => {
  await listen<boolean>('sync-passthrough', (event) => {
    isPassthrough.value = event.payload;
  });
});

const onPrev = () => emit('action-bar-prev');
const onNext = () => emit('action-bar-next');
const startDrag = () => invoke('start_drag');
</script>

<template>
  <div 
    class="w-full h-full flex items-center justify-center p-0 m-0 bg-poe-dark-gem border border-poe-tarnished-bronze opacity-40 hover:opacity-100 hover:border-poe-gold transition-all duration-200 cursor-move"
    style="border-radius: 4px;"
    @mousedown="startDrag"
  >
    <div class="flex gap-1 w-full h-full items-center justify-between px-2 text-poe-desaturated-gold hover:text-poe-gold transition-colors duration-200 shadow-text-glow font-bold text-lg select-none">
      <div 
        @click="onPrev"
        @mousedown.stop
        class="flex-1 h-full flex items-center justify-center hover:scale-110 active:scale-75 active:text-white active:brightness-150 transition-all cursor-pointer"
      >
        &#x2039;
      </div>
      <div class="w-[1px] h-3/5 bg-poe-tarnished-bronze/50 flex-shrink-0 mx-1"></div>
      <div 
        @click="onNext"
        @mousedown.stop
        class="flex-1 h-full flex items-center justify-center hover:scale-110 active:scale-75 active:text-white active:brightness-150 transition-all cursor-pointer"
      >
        &#x203A;
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 确保内联微小组件能够完美充满 tauri 配置的 64x28 宽高 */
</style>
