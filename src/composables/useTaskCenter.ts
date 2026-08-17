/**
 * 后台任务中心
 *
 * 统一跟踪长时间运行的后台操作（批量解压、批量扫描封面等），
 * 通过监听 Rust 端发射的进度事件自动创建/更新/完成任务，
 * 供 TaskCenter.vue 悬浮面板展示。
 */
import { ref, computed } from "vue";
import { listen } from "@tauri-apps/api/event";

export interface Task {
  id: number;
  /** 用于去重的事件类型标识 */
  key: string;
  /** i18n 标签键 */
  labelKey: string;
  /** 附加信息（如当前处理的文件名） */
  detail: string;
  current: number;
  total: number;
  status: "running" | "done" | "error";
  /** 失败原因（status === "error" 时存在） */
  error?: string;
}

const tasks = ref<Task[]>([]);
let nextId = 0;
let initialized = false;

/** 完成后延迟移除任务，让用户能看到完成状态 */
function scheduleRemove(id: number) {
  setTimeout(() => {
    tasks.value = tasks.value.filter((t) => t.id !== id);
  }, 2500);
}

/**
 * 将指定 key 的后台任务标记为失败
 *
 * 若该 key 存在运行中的任务，则将其置为 error 并记录失败原因；
 * 若不存在（例如任务尚未收到任何进度事件就已失败），
 * 则创建一个 error 状态的任务，保证失败对用户可见。
 * 失败任务不会自动移除，由用户手动关闭。
 */
export function failTask(key: string, message: string, labelKey?: string) {
  let task = tasks.value.find((t) => t.key === key && t.status === "running");
  if (!task) {
    task = { id: nextId++, key, labelKey: labelKey ?? "task.title", detail: "", current: 0, total: 0, status: "running" };
    tasks.value.push(task);
  }
  if (labelKey) task.labelKey = labelKey;
  task.status = "error";
  task.error = message;
}

/** 移除指定任务（用于用户关闭失败任务） */
export function dismissTask(id: number) {
  tasks.value = tasks.value.filter((t) => t.id !== id);
}

/** 批量移除所有已结束（完成/失败）的任务；运行中的任务不受影响 */
export function dismissFinishedTasks() {
  tasks.value = tasks.value.filter((t) => t.status === "running");
}

function upsert(key: string, labelKey: string, detail: string, current: number, total: number) {
  let task = tasks.value.find((t) => t.key === key && t.status === "running");
  if (!task) {
    task = { id: nextId++, key, labelKey, detail, current, total, status: "running" };
    tasks.value.push(task);
  }
  task.labelKey = labelKey;
  task.detail = detail;
  task.total = total;
  task.current = current;
  if (total > 0 && current >= total) {
    task.status = "done";
    scheduleRemove(task.id);
  }
}

/**
 * 初始化任务中心事件监听（幂等，仅首次生效）
 * 应在应用挂载时调用一次
 */
export async function initTaskCenter() {
  if (initialized) return;
  initialized = true;
  await listen<{ current: number; total: number; name?: string }>("extract-progress", (e) => {
    upsert("extract", "task.extracting", e.payload.name ?? "", e.payload.current, e.payload.total);
  });
  await listen<{ current: number; total: number }>("scan-covers-progress", (e) => {
    upsert("scan-covers", "task.scanningCovers", "", e.payload.current, e.payload.total);
  });
}

export function useTaskCenter() {
  const activeCount = computed(() => tasks.value.filter((t) => t.status === "running").length);
  return { tasks, activeCount };
}
