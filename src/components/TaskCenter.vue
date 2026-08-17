<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useTaskCenter, dismissTask, dismissFinishedTasks, type Task } from "../composables/useTaskCenter";

const { t } = useI18n();
const { tasks, activeCount } = useTaskCenter();
const collapsed = ref(false);

/** 已结束（完成/失败）的任务数，大于 0 时显示批量关闭按钮 */
const finishedCount = computed(() => tasks.value.filter((task) => task.status !== "running").length);

function percent(task: Task): number {
  return task.total > 0 ? Math.min(100, Math.round((task.current / task.total) * 100)) : 0;
}
</script>

<template>
  <transition name="task-fade">
    <div
      v-if="tasks.length"
      class="fixed bottom-6 right-6 z-[90] w-80 rounded-2xl bg-modal-bg border border-border-light shadow-2xl overflow-hidden"
    >
      <!-- Header -->
      <button
        class="w-full flex items-center justify-between px-4 py-3 hover:bg-primary-50 transition-colors"
        @click="collapsed = !collapsed"
      >
        <span class="flex items-center gap-2 text-sm font-medium text-text-main">
          <svg
            class="animate-spin"
            :class="{ 'opacity-0': activeCount === 0 }"
            width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5"
          >
            <circle cx="7" cy="7" r="5.5" stroke-opacity="0.25" />
            <path d="M12.5 7a5.5 5.5 0 00-5.5-5.5" class="text-primary-500" />
          </svg>
          {{ t("task.title") }}
          <span v-if="activeCount > 0" class="text-xs text-primary-500">({{ activeCount }})</span>
        </span>
        <span class="flex items-center gap-2">
          <button
            v-if="finishedCount > 0"
            class="text-xs text-text-sub hover:text-text-main transition-colors"
            :title="t('task.clearAll')"
            @click.stop="dismissFinishedTasks()"
          >{{ t("task.clearAll") }}</button>
          <span class="text-text-sub text-xs">{{ collapsed ? "▴" : "▾" }}</span>
        </span>
      </button>

      <!-- Task list -->
      <div v-show="!collapsed" class="px-4 pb-3 space-y-3 max-h-64 overflow-y-auto">
        <div v-for="task in tasks" :key="task.id" class="space-y-1.5">
          <div class="flex items-center justify-between gap-2">
            <span class="text-xs text-text-main truncate flex-1">
              {{ t(task.labelKey) }}
              <span v-if="task.detail" class="text-text-sub">{{ task.detail }}</span>
            </span>
            <span class="text-xs shrink-0" :class="task.status === 'error' ? 'text-red-500' : 'text-text-sub'">
              <template v-if="task.status === 'done'">{{ t("task.done") }}</template>
              <template v-else-if="task.status === 'error'">
                {{ t("task.error") }}
                <button
                  class="ml-1 text-text-sub hover:text-text-main"
                  :title="t('common.close')"
                  @click="dismissTask(task.id)"
                >✕</button>
              </template>
              <template v-else>{{ task.current }}/{{ task.total }}</template>
            </span>
          </div>
          <p v-if="task.status === 'error' && task.error" class="text-[11px] text-red-500/90 break-all line-clamp-2" :title="task.error">
            {{ task.error }}
          </p>
          <div class="h-1.5 rounded-full bg-input-bg overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-300"
              :class="task.status === 'done' ? 'bg-emerald-500' : task.status === 'error' ? 'bg-red-500' : 'bg-primary-500'"
              :style="{ width: (task.status === 'error' ? 100 : percent(task)) + '%' }"
            />
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.task-fade-enter-active,
.task-fade-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}
.task-fade-enter-from,
.task-fade-leave-to {
  opacity: 0;
  transform: translateY(12px);
}
</style>
