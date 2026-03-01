import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { LazyStore } from '@tauri-apps/plugin-store';

const appStore = new LazyStore('store.json');

export const useOverlayStore = defineStore('overlay', {
    state: () => ({
        // 基础配置
        fontSize: 14,
        maxNotesLines: 7,

        // 状态切换
        isMinimal: false,
        isPassthrough: false,

        // 进度数据
        currentStepIndex: 0,
        completedStepIds: [] as string[],
        steps: [] as { id: string, chapter: string, text: string }[],
        notes: "等待解析文档...",

        // 持久化数据
        lastDocumentPath: ""
    }),
    getters: {
        chapterList: (state): { name: string, startIndex: number }[] => {
            const list: { name: string, startIndex: number }[] = [];
            const seen = new Set<string>();
            state.steps.forEach((step, index) => {
                if (!seen.has(step.chapter)) {
                    seen.add(step.chapter);
                    list.push({ name: step.chapter, startIndex: index });
                }
            });
            return list;
        }
    },
    actions: {
        async initializeStore() {
            try {
                // 从本地加载上一次关闭前的缓存数据
                const path = await appStore.get<string>('lastDocumentPath');
                const targetIndex = await appStore.get<number>('lastTargetIndex');

                if (path) {
                    this.lastDocumentPath = path;
                    // 如果有历史路径，立即请求后端拉取并监听
                    await invoke('start_file_watcher', { path });

                    // 注意：如果后端解析完立即通过 tauri 事件发回来，updateParsedDocument 
                    // 此时还不知道初始的 index 该设置多少，所以我们暂存到本类实例上
                    if (typeof targetIndex === 'number') {
                        // 我们借用 `currentStepIndex` 预热，稍后 updateParsedDocument 看到没有 oldChapter 时会当成新数据取用
                        this.currentStepIndex = targetIndex;
                    }
                }
            } catch (err) {
                console.error("Failed to load store persistence:", err);
            }
        },
        async saveCurrentProgress() {
            // 防抖存储防止频繁写入硬盘
            await appStore.set('lastTargetIndex', this.currentStepIndex);
            await appStore.save();
        },
        async setDocumentPath(path: string) {
            this.lastDocumentPath = path;
            await appStore.set('lastDocumentPath', path);
            await appStore.save();
        },
        async toggleMinimal() {
            this.isMinimal = !this.isMinimal;
            await invoke('set_shadow', { shadow: !this.isMinimal });
            // 极简模式关闭时，鼠标穿透也必须关闭
            if (!this.isMinimal && this.isPassthrough) {
                this.isPassthrough = false;
                await invoke('set_passthrough', { passthrough: false });
                await emit('sync-passthrough', false);
            }
        },
        async togglePassthrough() {
            this.isPassthrough = !this.isPassthrough;
            await invoke('set_passthrough', { passthrough: this.isPassthrough });
            await emit('sync-passthrough', this.isPassthrough);
        },
        zoomIn() {
            if (this.fontSize < 24) this.fontSize += 1;
        },
        zoomOut() {
            if (this.fontSize > 10) this.fontSize -= 1;
        },
        jumpToStep(index: number) {
            if (index >= 0 && index < this.steps.length) {
                this.currentStepIndex = index;
                this.completedStepIds = this.steps.slice(0, this.currentStepIndex).map(s => s.id);
                this.saveCurrentProgress();
            }
        },
        nextStep() {
            if (this.currentStepIndex < this.steps.length - 1) {
                this.currentStepIndex++;
                // 严格保证 completedStepIds 包含当前索引之前的所有 ID
                this.completedStepIds = this.steps.slice(0, this.currentStepIndex).map(s => s.id);
                this.saveCurrentProgress();
            }
        },
        prevStep() {
            if (this.currentStepIndex > 0) {
                this.currentStepIndex--;
                // 严格保证 completedStepIds 包含当前索引之前的所有 ID
                this.completedStepIds = this.steps.slice(0, this.currentStepIndex).map(s => s.id);
                this.saveCurrentProgress();
            }
        },
        updateParsedDocument(payload: { notes: string, steps: { id: string, chapter: string, text: string }[] }) {
            // 1. 记录热更新前瞬时的“高亮锚点”
            let oldActiveChapter: string | null = null;
            let oldActiveText: string | null = null;
            let oldActiveIndexInChapter = 0;

            if (this.steps.length > 0 && this.currentStepIndex >= 0 && this.currentStepIndex < this.steps.length) {
                const currentStep = this.steps[this.currentStepIndex];
                if (currentStep) {
                    oldActiveChapter = currentStep.chapter;
                    oldActiveText = currentStep.text;
                    // 计算该节点在当前章节中是第几条（相对序号）
                    oldActiveIndexInChapter = this.steps
                        .filter(s => s.chapter === oldActiveChapter)
                        .findIndex(s => s.id === currentStep.id);
                }
            }

            // 更新数据源
            this.notes = payload.notes;
            this.steps = payload.steps;

            // 2. 在新数据中寻找这个锚点
            let newTargetIndex = 0; // 默认跳回第一条
            if (this.steps.length > 0) {
                if (oldActiveChapter) {
                    // 先尝试找同样的章节和一模一样的内容（针对在上面随便改了东西的情况）
                    const exactMatchIndex = this.steps.findIndex(s => s.chapter === oldActiveChapter && s.text === oldActiveText);

                    if (exactMatchIndex !== -1) {
                        newTargetIndex = exactMatchIndex;
                    } else {
                        // 如果没找到一模一样的文字（说明用户改的就是当前行），退而求其次
                        // 找到他原来的那个章节的所有条目，按照他原来的相对序号插回去
                        const newChapterSteps = this.steps
                            .map((s, idx) => ({ s, idx }))
                            .filter(x => x.s.chapter === oldActiveChapter);

                        if (newChapterSteps && newChapterSteps.length > 0) {
                            // 防止越界，比如他删了条目
                            const safeIndex = Math.min(Math.max(0, oldActiveIndexInChapter), newChapterSteps.length - 1);
                            const targetStep = newChapterSteps[safeIndex];
                            if (targetStep) {
                                newTargetIndex = targetStep.idx;
                            } else {
                                newTargetIndex = Math.min(this.currentStepIndex, Math.max(0, this.steps.length - 1));
                            }
                        } else {
                            // 如果整个章节都被他删了...那就留在原来的绝对序号上吧
                            newTargetIndex = Math.min(this.currentStepIndex, Math.max(0, this.steps.length - 1));
                        }
                    }
                } else {
                    // 如果是冷启动，保留从持久化那边传过来的缓存值
                    newTargetIndex = Math.min(this.currentStepIndex, Math.max(0, this.steps.length - 1));
                }
            }

            this.currentStepIndex = newTargetIndex;

            // 3. 重塑已读进度 (把新位置之前的所有 ID 标记为已完成)
            this.completedStepIds = this.steps.slice(0, newTargetIndex).map(s => s.id);
            this.saveCurrentProgress();
        }
    }
});
