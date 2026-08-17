import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock Tauri API（wrapper 内部依赖 @tauri-apps/api/core 的原始 invoke）
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { invoke, InvokeError, normalizeInvokeError } from "../invoke";
import { failTask, dismissTask, useTaskCenter } from "../../composables/useTaskCenter";

const mockedInvoke = vi.mocked(tauriInvoke);

describe("utils/invoke 统一包装", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("成功时透传后端返回值", async () => {
    mockedInvoke.mockResolvedValueOnce([1, 2, 3]);
    const result = await invoke<number[]>("get_all_games");
    expect(result).toEqual([1, 2, 3]);
    expect(mockedInvoke).toHaveBeenCalledWith("get_all_games", undefined);
  });

  it("将裸字符串错误归一化为携带命令名的 InvokeError", async () => {
    mockedInvoke.mockRejectedValueOnce("database is locked");
    await expect(invoke("delete_game", { id: 1 })).rejects.toMatchObject({
      name: "InvokeError",
      command: "delete_game",
      message: "[delete_game] database is locked",
    });
  });

  it("归一化 Error 实例与其他类型错误", () => {
    const fromError = normalizeInvokeError("launch_game", new Error("exe not found"));
    expect(fromError).toBeInstanceOf(InvokeError);
    expect(fromError.message).toBe("[launch_game] exe not found");

    const fromObject = normalizeInvokeError("save_settings", { code: 500 });
    expect(fromObject.message).toBe('[save_settings] {"code":500}');

    const fromNull = normalizeInvokeError("get_settings", null);
    expect(fromNull.message).toBe("[get_settings] Unknown error");
  });

  it("提供 taskKey 时失败会写入任务中心的 error 状态", async () => {
    mockedInvoke.mockRejectedValueOnce("7z not found");
    const { tasks } = useTaskCenter();

    await expect(
      invoke("batch_scan_covers", { gameIds: [1] }, { taskKey: "scan-covers" })
    ).rejects.toBeInstanceOf(InvokeError);

    const task = tasks.value.find((t) => t.key === "scan-covers");
    expect(task).toBeDefined();
    expect(task?.status).toBe("error");
    expect(task?.error).toBe("[batch_scan_covers] 7z not found");
  });
});

describe("useTaskCenter 失败收口", () => {
  it("failTask 将运行中任务标记为 error 且可被 dismiss", () => {
    const { tasks } = useTaskCenter();

    failTask("extract", "[batch_extract_games] wrong password", "task.extracting");
    const task = tasks.value.find((t) => t.key === "extract");
    expect(task?.status).toBe("error");
    expect(task?.labelKey).toBe("task.extracting");
    expect(task?.error).toBe("[batch_extract_games] wrong password");

    dismissTask(task!.id);
    expect(tasks.value.find((t) => t.key === "extract")).toBeUndefined();
  });
});
