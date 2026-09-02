import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

/**
 * 统一 invoke 收口守卫（AGENTS.md 关键约定）
 *
 * 前端所有 Tauri 命令调用必须经 src/utils/invoke.ts 包装，以获得
 * 错误归一化（InvokeError）与任务中心失败可见性。本测试扫描 src/ 下
 * 全部生产代码，禁止再从 @tauri-apps/api/core 直接 import invoke。
 */

const SRC_ROOT = join(__dirname, "..", "..");
/** 允许直接引用裸 invoke 的文件：包装实现本身 */
const ALLOWED = ["utils/invoke.ts"];
const BARE_INVOKE_IMPORT =
  /import\s*\{[^}]*\binvoke\b[^}]*\}\s*from\s*["']@tauri-apps\/api\/core["']/;

function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      if (entry === "__tests__") continue;
      collectSourceFiles(full, out);
    } else if (/\.(ts|vue)$/.test(entry)) {
      out.push(full);
    }
  }
  return out;
}

describe("统一 invoke 收口约定", () => {
  it("守卫规则自身能命中违规写法、不误伤合规写法", () => {
    expect(BARE_INVOKE_IMPORT.test('import { invoke } from "@tauri-apps/api/core";')).toBe(true);
    expect(
      BARE_INVOKE_IMPORT.test('import { invoke, convertFileSrc } from "@tauri-apps/api/core";')
    ).toBe(true);
    expect(BARE_INVOKE_IMPORT.test('import { invoke } from "../utils/invoke";')).toBe(false);
    expect(
      BARE_INVOKE_IMPORT.test('import { convertFileSrc } from "@tauri-apps/api/core";')
    ).toBe(false);
  });
  it("src/ 下不存在绕过包装的裸 invoke 导入", () => {
    const files = collectSourceFiles(SRC_ROOT).map((f) =>
      relative(SRC_ROOT, f).replace(/\\/g, "/")
    );
    // 扫描范围自检：路径写错会导致空扫描假通过
    expect(files.length).toBeGreaterThan(20);
    const offenders = files
      .filter((rel) => !ALLOWED.includes(rel))
      .filter((rel) =>
        BARE_INVOKE_IMPORT.test(readFileSync(join(SRC_ROOT, rel), "utf-8"))
      );
    expect(offenders).toEqual([]);
  });
});
