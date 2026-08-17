<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { useGameStore, loadImage } from "../stores/gameStore";
import type { Game } from "../types";

const { t } = useI18n();
const store = useGameStore();

const emit = defineEmits<{
  close: [];
}>();

type Phase = "tags" | "anim" | "result";
const phase = ref<Phase>("tags");
const excluded = ref(new Set<number>());
const animating = ref(false);
const current = ref<Game | null>(null);
const coverUrl = ref("");
const deck = ref<Game[]>([]);
// 抽取动画阶段快速滚动的候选游戏名，营造转盘感
const flashName = ref("");
let flashTimer: ReturnType<typeof setInterval> | null = null;

// —— 标签散布布局（伪随机位置，仅打开时生成一次）——
interface ScatterTag {
  id: number;
  name: string;
  top: number;
  left: number;
  rotate: number;
  size: number;
}
function mulberry32(seed: number) {
  let a = seed;
  return () => {
    a |= 0; a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const scatterTags = computed<ScatterTag[]>(() => {
  const rand = mulberry32(20260803);
  return store.tags.map((tag) => ({
    id: tag.id,
    name: tag.name,
    top: Math.round(6 + rand() * 76),
    left: Math.round(6 + rand() * 78),
    rotate: Math.round((rand() * 2 - 1) * 14),
    size: Math.round(12 + rand() * 5),
  }));
});

// —— 候选池与牌堆 ——
function buildDeck() {
  const pool = store.games.filter(
    (g) => !(store.gameTags.get(g.id) ?? []).some((tg) => excluded.value.has(tg.id))
  );
  // Fisher-Yates 洗牌
  const arr = [...pool];
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  deck.value = arr;
}

// 没有任何标签时跳过引导页，直接进入抽取
const hasTags = computed(() => store.tags.length > 0);
if (!hasTags.value) {
  phase.value = "anim";
  buildDeck();
  draw();
} else {
  buildDeck();
}

const candidateCount = computed(() => deck.value.length);
const deckExhausted = computed(() => deck.value.length === 0);

function toggleTag(id: number) {
  const next = new Set(excluded.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  excluded.value = next;
  buildDeck();
}

function startDraw() {
  phase.value = "anim";
  draw();
}

// 换一个：必须先切回动画阶段，否则结果页会闪现"已抽空"兜底分支
function redraw() {
  phase.value = "anim";
  draw();
}

function draw() {
  animating.value = true;
  current.value = null;
  coverUrl.value = "";
  // 名字快速滚动（含已抽出的，营造转盘感）
  flashTimer = setInterval(() => {
    if (store.games.length === 0) return;
    flashName.value = store.games[Math.floor(Math.random() * store.games.length)].name;
  }, 80);
  setTimeout(() => {
    if (flashTimer) { clearInterval(flashTimer); flashTimer = null; }
    animating.value = false;
    flashName.value = "";
    if (deck.value.length === 0) {
      phase.value = "result";
      return;
    }
    const g = deck.value.shift()!;
    current.value = g;
    coverUrl.value = g.cover_path ? loadImage(g.cover_path) : "";
    phase.value = "result";
  }, 1300);
}

function reshuffle() {
  buildDeck();
  phase.value = "anim";
  draw();
}

async function launchCurrent() {
  if (!current.value) return;
  await store.launchGame(current.value.id);
  emit("close");
}

// 对话框关闭时清理定时器
onUnmounted(() => {
  if (flashTimer) clearInterval(flashTimer);
});
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-md"
    @click.self="emit('close')"
  >
    <div class="modal-panel relative bg-modal-bg rounded-3xl shadow-2xl w-[600px] max-h-[84vh] overflow-hidden flex flex-col">
      <!-- 顶部渐变装饰带 -->
      <div class="absolute inset-x-0 top-0 h-28 bg-gradient-to-b from-primary-500/15 via-primary-500/5 to-transparent pointer-events-none"></div>

      <!-- Header -->
      <div class="relative flex items-center justify-between px-8 py-6 border-b border-border-light shrink-0">
        <h2 class="text-lg font-bold text-text-main">🎲 {{ t('random.title') }}</h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <!-- 阶段一：标签星云（凌乱散布，点击排除） -->
      <div v-if="phase === 'tags'" class="relative p-8 flex flex-col gap-5">
        <p class="text-sm text-text-sub">{{ t('random.pickTagsHint') }}</p>
        <div class="tag-field relative h-[300px] rounded-2xl overflow-hidden border border-border-light">
          <!-- 装饰星光 -->
          <span class="field-star" style="top: 14%; left: 22%">✦</span>
          <span class="field-star" style="top: 68%; left: 12%; animation-delay: 1.2s">✧</span>
          <span class="field-star" style="top: 30%; left: 88%; animation-delay: 0.6s">✦</span>
          <span class="field-star" style="top: 82%; left: 74%; animation-delay: 1.8s">✧</span>
          <span
            v-for="st in scatterTags"
            :key="st.id"
            class="scatter-tag absolute px-3.5 py-1.5 rounded-full cursor-pointer select-none border backdrop-blur-sm"
            :class="excluded.has(st.id) ? 'is-excluded' : 'is-active'"
            :style="{
              top: st.top + '%',
              left: st.left + '%',
              fontSize: st.size + 'px',
              transform: `translate(-50%, -50%) rotate(${st.rotate}deg)`,
            }"
            @click="toggleTag(st.id)"
          >
            {{ st.name }}
          </span>
          <p v-if="scatterTags.length === 0" class="absolute inset-0 flex items-center justify-center text-sm text-text-sub">
            {{ t('random.noTags') }}
          </p>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-sm text-text-sub">{{ t('random.candidateCount', { count: candidateCount }) }}</span>
          <button
            class="px-6 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 hover:scale-[1.03] active:scale-[0.97] transition-all disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:scale-100 shadow-lg shadow-primary-500/30"
            :disabled="candidateCount === 0"
            @click="startDraw"
          >
            🎲 {{ t('random.start') }}
          </button>
        </div>
      </div>

      <!-- 阶段二：名字转盘动画 -->
      <div v-else-if="phase === 'anim'" class="relative p-8 flex flex-col items-center justify-center gap-7 h-[380px] overflow-hidden">
        <span class="field-star" style="top: 18%; left: 18%">✦</span>
        <span class="field-star" style="top: 26%; left: 82%; animation-delay: 0.8s">✧</span>
        <span class="field-star" style="top: 76%; left: 24%; animation-delay: 1.4s">✧</span>
        <span class="field-star" style="top: 70%; left: 78%; animation-delay: 0.4s">✦</span>
        <!-- 发光脉动圆环 + 骰子 -->
        <div class="relative flex items-center justify-center">
          <div class="pulse-ring absolute w-32 h-32 rounded-full border-2 border-primary-400/50"></div>
          <div class="pulse-ring absolute w-32 h-32 rounded-full border-2 border-primary-400/30" style="animation-delay: 0.7s"></div>
          <div class="dice-glow w-24 h-24 rounded-2xl bg-gradient-to-br from-primary-400 to-primary-600 flex items-center justify-center text-5xl shadow-2xl shadow-primary-500/40" :class="animating ? 'dice-spin' : ''">
            🎲
          </div>
        </div>
        <!-- 名字快速滚动 -->
        <div class="h-8 overflow-hidden flex items-center">
          <span class="text-xl font-bold text-text-main blur-[0.5px] tracking-wide">{{ flashName || t('random.drawing') }}</span>
        </div>
        <p class="text-xs text-text-sub">{{ t('random.drawing') }}</p>
      </div>

      <!-- 阶段三：结果卡片（聚光翻牌） -->
      <div v-else class="relative p-8 flex flex-col items-center gap-5">
        <template v-if="current">
          <!-- 聚光背景 -->
          <div class="spotlight absolute inset-x-0 top-0 h-64 pointer-events-none"></div>
          <span class="field-star" style="top: 12%; left: 20%">✦</span>
          <span class="field-star" style="top: 18%; left: 80%; animation-delay: 0.5s">✧</span>
          <span class="field-star" style="top: 46%; left: 10%; animation-delay: 1s">✧</span>
          <span class="field-star" style="top: 40%; left: 90%; animation-delay: 1.5s">✦</span>
          <div class="result-card relative w-[210px] rounded-2xl p-[3px] bg-gradient-to-b from-primary-400/70 via-primary-500/30 to-transparent shadow-2xl shadow-primary-500/20">
            <div class="rounded-[13px] overflow-hidden bg-input-bg">
              <img
                v-if="coverUrl"
                :src="coverUrl"
                class="w-full aspect-square object-contain bg-gradient-to-b from-input-bg to-primary-50/50"
              />
              <div v-else class="w-full aspect-square flex items-center justify-center text-5xl text-text-sub/25 bg-gradient-to-b from-input-bg to-primary-50">
                🎮
              </div>
            </div>
          </div>
          <h3 class="result-text text-xl font-bold text-text-main text-center">{{ current.name }}</h3>
          <div v-if="(store.gameTags.get(current.id) ?? []).length" class="result-text flex flex-wrap justify-center gap-1.5 -mt-2">
            <span
              v-for="tg in store.gameTags.get(current.id)"
              :key="tg.id"
              class="px-2 py-0.5 rounded-full text-xs bg-primary-50 text-primary-600 border border-primary-200"
            >
              {{ tg.name }}
            </span>
          </div>
          <div class="result-text flex items-center gap-3 mt-2">
            <button
              class="px-6 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 hover:scale-[1.03] active:scale-[0.97] transition-all shadow-lg shadow-primary-500/30"
              @click="launchCurrent"
            >
              ▶ {{ t('random.launch') }}
            </button>
            <button
              class="px-6 py-2.5 rounded-xl border border-border-medium text-sm text-text-sub hover:bg-input-bg hover:scale-[1.03] active:scale-[0.97] transition-all disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:scale-100"
              :disabled="deckExhausted"
              @click="redraw"
            >
              🔄 {{ t('random.redraw') }}
            </button>
          </div>
          <p v-if="deckExhausted" class="text-xs text-text-sub">{{ t('random.exhausted') }}</p>
        </template>
        <template v-else>
          <p class="text-sm text-text-sub py-8">{{ t('random.exhausted') }}</p>
        </template>
        <button
          v-if="deckExhausted"
          class="px-4 py-2 rounded-xl border border-border-medium text-sm text-text-sub hover:bg-input-bg transition-colors"
          @click="reshuffle"
        >
          {{ t('random.reshuffle') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* —— 标签星云背景 —— */
.tag-field {
  background:
    radial-gradient(ellipse 80% 60% at 30% 20%, var(--color-primary-500, #8b5cf6) 0%, transparent 55%),
    radial-gradient(ellipse 70% 55% at 75% 80%, var(--color-primary-400, #a78bfa) 0%, transparent 55%),
    var(--color-input-bg, rgba(0, 0, 0, 0.04));
  background-blend-mode: soft-light, soft-light, normal;
}
.scatter-tag {
  transition: all 0.25s cubic-bezier(0.2, 0.8, 0.3, 1.2);
  animation: tag-float 3.2s ease-in-out infinite alternate;
}
.scatter-tag.is-active {
  background: color-mix(in srgb, var(--color-modal-bg, #fff) 75%, transparent);
  border-color: var(--color-primary-200, #ddd6fe);
  color: var(--color-primary-600, #7c3aed);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}
.scatter-tag.is-active:hover {
  box-shadow: 0 6px 20px var(--color-primary-500, #8b5cf6), 0 0 0 2px var(--color-primary-400, #a78bfa);
  filter: brightness(1.06);
}
.scatter-tag.is-excluded {
  background: transparent;
  border-color: var(--color-border-light, #e5e7eb);
  color: color-mix(in srgb, var(--color-text-sub, #888) 45%, transparent);
  text-decoration: line-through;
  filter: grayscale(1);
  opacity: 0.65;
}
@keyframes tag-float {
  from { margin-top: 0; }
  to { margin-top: -5px; }
}

/* —— 装饰星光 —— */
.field-star {
  position: absolute;
  color: var(--color-primary-400, #a78bfa);
  font-size: 13px;
  opacity: 0.5;
  pointer-events: none;
  animation: star-twinkle 2.4s ease-in-out infinite;
}
@keyframes star-twinkle {
  0%, 100% { opacity: 0.2; transform: scale(0.8); }
  50% { opacity: 0.85; transform: scale(1.15); }
}

/* —— 抽取动画 —— */
.pulse-ring {
  animation: ring-pulse 1.4s ease-out infinite;
}
@keyframes ring-pulse {
  0% { transform: scale(0.6); opacity: 0.9; }
  100% { transform: scale(1.9); opacity: 0; }
}
.dice-spin {
  animation: dice-spin 1.15s cubic-bezier(0.45, 0, 0.55, 1) infinite;
}
@keyframes dice-spin {
  0% { transform: rotate(0deg) scale(1); }
  25% { transform: rotate(90deg) scale(1.12) translateY(-4px); }
  50% { transform: rotate(180deg) scale(1); }
  75% { transform: rotate(270deg) scale(1.12) translateY(-4px); }
  100% { transform: rotate(360deg) scale(1); }
}
.dice-glow {
  background-size: 200% 200%;
}

/* —— 结果聚光与翻牌 —— */
.spotlight {
  background: radial-gradient(ellipse 55% 70% at 50% 20%, color-mix(in srgb, var(--color-primary-400, #a78bfa) 22%, transparent), transparent 70%);
}
.result-card {
  animation: card-in 0.5s cubic-bezier(0.2, 0.8, 0.3, 1.15);
  transform-style: preserve-3d;
}
@keyframes card-in {
  0% { transform: perspective(800px) rotateY(90deg) scale(0.6); opacity: 0; }
  60% { transform: perspective(800px) rotateY(-8deg) scale(1.04); opacity: 1; }
  100% { transform: perspective(800px) rotateY(0deg) scale(1); }
}
.result-text {
  animation: text-rise 0.45s 0.2s cubic-bezier(0.2, 0.8, 0.3, 1.2) backwards;
}
@keyframes text-rise {
  from { transform: translateY(12px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
