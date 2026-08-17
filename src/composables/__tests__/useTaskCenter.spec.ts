import { describe, it, expect, beforeAll, vi } from "vitest";
import { initTaskCenter, useTaskCenter } from "../useTaskCenter";

// 捕获事件监听回调，便于手动触发模拟事件
const handlers: Record<string, (e: { payload: unknown }) => void> = {};
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, handler: (e: { payload: unknown }) => void) => {
    handlers[event] = handler;
    return () => {};
  }),
}));

function emit(event: string, payload: unknown) {
  handlers[event]({ payload });
}

describe("useTaskCenter", () => {
  beforeAll(async () => {
    vi.useFakeTimers();
    await initTaskCenter();
  });

  it("registers extract and scan-covers listeners", () => {
    expect(handlers["extract-progress"]).toBeDefined();
    expect(handlers["scan-covers-progress"]).toBeDefined();
  });

  it("creates a running task and updates progress in place", () => {
    const { tasks, activeCount } = useTaskCenter();
    expect(tasks.value).toHaveLength(0);

    emit("extract-progress", { current: 1, total: 3, name: "a.zip" });
    expect(tasks.value).toHaveLength(1);
    expect(tasks.value[0]).toMatchObject({
      key: "extract",
      labelKey: "task.extracting",
      detail: "a.zip",
      current: 1,
      total: 3,
      status: "running",
    });
    expect(activeCount.value).toBe(1);

    // 相同 key 的事件应更新同一任务而非新建
    emit("extract-progress", { current: 2, total: 3, name: "b.zip" });
    expect(tasks.value).toHaveLength(1);
    expect(tasks.value[0].current).toBe(2);
    expect(tasks.value[0].detail).toBe("b.zip");
  });

  it("marks task done at completion and removes it after delay", () => {
    const { tasks, activeCount } = useTaskCenter();

    emit("extract-progress", { current: 3, total: 3, name: "c.zip" });
    expect(tasks.value[0].status).toBe("done");
    expect(activeCount.value).toBe(0);

    // 完成后延迟 2500ms 移除
    vi.advanceTimersByTime(2600);
    expect(tasks.value).toHaveLength(0);
  });

  it("tracks scan-covers progress independently", () => {
    const { tasks } = useTaskCenter();

    emit("scan-covers-progress", { current: 5, total: 10 });
    expect(tasks.value).toHaveLength(1);
    expect(tasks.value[0]).toMatchObject({
      key: "scan-covers",
      labelKey: "task.scanningCovers",
      current: 5,
      total: 10,
      status: "running",
    });

    emit("scan-covers-progress", { current: 10, total: 10 });
    expect(tasks.value[0].status).toBe("done");
    vi.advanceTimersByTime(2600);
    expect(tasks.value).toHaveLength(0);
  });
});
