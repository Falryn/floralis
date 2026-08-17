<script setup lang="ts">
import { ref, watchEffect, computed } from "vue";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import { formatPlayTime as fmtPlayTime } from "../utils/format";

const { t } = useI18n();
const store = useGameStore();

const currentDate = ref(new Date());
const calendarData = ref<Map<string, number>>(new Map());

const year = computed(() => currentDate.value.getFullYear());
const month = computed(() => currentDate.value.getMonth() + 1);

const daysInMonth = computed(() => {
  return new Date(year.value, month.value, 0).getDate();
});

const firstDayOfWeek = computed(() => {
  return new Date(year.value, month.value - 1, 1).getDay();
});

const monthName = computed(() => {
  return t('calendar.yearMonth', { year: year.value, month: month.value });
});

watchEffect(async () => {
  const data = await store.getPlayCalendar(year.value, month.value);
  const map = new Map<string, number>();
  for (const d of data) {
    map.set(d.date, d.duration);
  }
  calendarData.value = map;
});

function prevMonth() {
  const d = new Date(currentDate.value);
  d.setMonth(d.getMonth() - 1);
  currentDate.value = d;
}

function nextMonth() {
  const d = new Date(currentDate.value);
  d.setMonth(d.getMonth() + 1);
  currentDate.value = d;
}

function formatDuration(seconds: number): string {
  return fmtPlayTime(seconds, t, "calendar");
}

function getHeatColor(seconds: number): string {
  if (seconds === 0) return "bg-gray-100";
  if (seconds < 1800) return "bg-green-200";
  if (seconds < 3600) return "bg-green-300";
  if (seconds < 7200) return "bg-green-400";
  return "bg-green-500";
}

const calendarDays = computed(() => {
  const days: { day: number; date: string; duration: number }[] = [];
  for (let i = 1; i <= daysInMonth.value; i++) {
    const dateStr = `${year.value}-${String(month.value).padStart(2, "0")}-${String(i).padStart(2, "0")}`;
    days.push({
      day: i,
      date: dateStr,
      duration: calendarData.value.get(dateStr) || 0,
    });
  }
  return days;
});
</script>

<template>
  <div class="bg-code-bg rounded-2xl p-4 space-y-3">
    <div class="flex justify-between items-center">
      <button class="text-text-sub hover:text-text-main transition-colors" @click="prevMonth">‹</button>
      <p class="text-xs text-text-sub font-medium">{{ monthName }}</p>
      <button class="text-text-sub hover:text-text-main transition-colors" @click="nextMonth">›</button>
    </div>
    <div class="grid grid-cols-7 gap-1 text-center">
      <div v-for="d in [t('calendar.sun'), t('calendar.mon'), t('calendar.tue'), t('calendar.wed'), t('calendar.thu'), t('calendar.fri'), t('calendar.sat')]" :key="d" class="text-[10px] text-text-sub/60 py-1">
        {{ d }}
      </div>
      <!-- Empty cells for days before the first day of month -->
      <div v-for="i in firstDayOfWeek" :key="'empty-' + i" class="aspect-square"></div>
      <!-- Calendar days -->
      <div
        v-for="d in calendarDays"
        :key="d.date"
        class="aspect-square rounded flex items-center justify-center text-[10px] cursor-default"
        :class="getHeatColor(d.duration)"
        :title="`${d.date}: ${d.duration ? formatDuration(d.duration) : t('calendar.notPlayed')}`"
      >
        {{ d.day }}
      </div>
    </div>
    <div class="flex items-center justify-end gap-1 pt-1">
      <span class="text-[10px] text-text-sub">{{ t('calendar.less') }}</span>
      <div class="w-3 h-3 rounded bg-gray-100"></div>
      <div class="w-3 h-3 rounded bg-green-200"></div>
      <div class="w-3 h-3 rounded bg-green-300"></div>
      <div class="w-3 h-3 rounded bg-green-400"></div>
      <div class="w-3 h-3 rounded bg-green-500"></div>
      <span class="text-[10px] text-text-sub">{{ t('calendar.more') }}</span>
    </div>
  </div>
</template>
