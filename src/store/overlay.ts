import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';

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
        notes: "等待解析文档..."
    }),
    actions: {
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
        nextStep() {
            if (this.currentStepIndex < this.steps.length) {
                const step = this.steps[this.currentStepIndex];
                if (step && !this.completedStepIds.includes(step.id)) {
                    this.completedStepIds.push(step.id);
                }
                this.currentStepIndex++;
            }
        },
        prevStep() {
            if (this.currentStepIndex > 0) {
                this.currentStepIndex--;
                const currStep = this.steps[this.currentStepIndex];
                if (currStep) {
                    this.completedStepIds = this.completedStepIds.filter(id => id !== currStep.id);
                }
            }
        },
        updateParsedDocument(payload: { notes: string, steps: { id: string, chapter: string, text: string }[] }) {
            this.notes = payload.notes;
            this.steps = payload.steps;

            // 根据 completedStepIds 恢复之前的进度
            // 找到第一个未处于 completedStepIds 中的 step，作为当前进度
            let foundIndex = this.steps.findIndex(s => !this.completedStepIds.includes(s.id));
            if (foundIndex === -1) {
                // 如果全都完成了，就停留在最后一条
                foundIndex = this.steps.length > 0 ? this.steps.length - 1 : 0;
            }
            this.currentStepIndex = foundIndex;
        }
    }
});
