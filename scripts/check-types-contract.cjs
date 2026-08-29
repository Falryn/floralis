/**
 * Rust 后端模型与前端类型定义的契约一致性检查
 *
 * 对比 src-tauri/src/models.rs、src-tauri/src/db/ 中的 pub struct 字段
 * 与 src/types.ts 中的 interface 字段。仅校验两侧同时存在的同名结构，
 * 字段名与顺序必须完全一致，否则以非零退出码失败。
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');

const RUST_FILES = [
  path.join(ROOT, 'src-tauri', 'src', 'models.rs'),
  // db 层按领域拆分为 db/*.rs，全部纳入契约校验
  ...fs
    .readdirSync(path.join(ROOT, 'src-tauri', 'src', 'db'))
    .filter((f) => f.endsWith('.rs'))
    .map((f) => path.join(ROOT, 'src-tauri', 'src', 'db', f)),
];
const TS_FILE = path.join(ROOT, 'src', 'types.ts');

/** 解析 Rust 文件中的 pub struct 及其 pub 字段（按声明顺序） */
function parseRustStructs(filePath) {
  const source = fs.readFileSync(filePath, 'utf8');
  const lines = source.split(/\r?\n/);
  const structs = new Map();
  let current = null;
  let depth = 0;

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (current === null) {
      const m = line.match(/^pub struct (\w+)\s*\{/);
      if (m) {
        current = m[1];
        depth = 1;
        structs.set(current, []);
      }
      continue;
    }
    // 统计花括号深度，仅处理顶层结构体字段
    depth += (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;
    if (depth <= 0) {
      current = null;
      continue;
    }
    if (depth === 1) {
      const fm = line.match(/^pub (\w+)\s*:/);
      if (fm) structs.get(current).push(fm[1]);
    }
  }
  return structs;
}

/** 解析 TypeScript 文件中的 export interface 及其字段（按声明顺序） */
function parseTsInterfaces(filePath) {
  const source = fs.readFileSync(filePath, 'utf8');
  const lines = source.split(/\r?\n/);
  const interfaces = new Map();
  let current = null;
  let depth = 0;

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (current === null) {
      const m = line.match(/^export interface (\w+)\s*\{/);
      if (m) {
        current = m[1];
        depth = 1;
        interfaces.set(current, []);
      }
      continue;
    }
    depth += (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;
    if (depth <= 0) {
      current = null;
      continue;
    }
    if (depth === 1 && !line.startsWith('//') && !line.startsWith('/*') && !line.startsWith('*')) {
      const fm = line.match(/^(\w+)\s*[?]?\s*:/);
      if (fm) interfaces.get(current).push(fm[1]);
    }
  }
  return interfaces;
}

function main() {
  const rustStructs = new Map();
  for (const f of RUST_FILES) {
    for (const [name, fields] of parseRustStructs(f)) {
      rustStructs.set(name, fields);
    }
  }
  const tsInterfaces = parseTsInterfaces(TS_FILE);

  // 仅校验两侧同时定义的同名结构（共享契约）
  const sharedNames = [...tsInterfaces.keys()].filter((name) => rustStructs.has(name));

  if (sharedNames.length === 0) {
    console.error('[contract-check] 未找到任何两侧共享的结构定义，检查失败');
    process.exit(1);
  }

  const errors = [];
  for (const name of sharedNames) {
    const rustFields = rustStructs.get(name);
    const tsFields = tsInterfaces.get(name);
    const rustSet = new Set(rustFields);
    const tsSet = new Set(tsFields);

    const missingInTs = rustFields.filter((f) => !tsSet.has(f));
    const missingInRust = tsFields.filter((f) => !rustSet.has(f));
    for (const f of missingInTs) errors.push(`${name}: 字段 "${f}" 存在于 Rust 但缺失于 types.ts`);
    for (const f of missingInRust) errors.push(`${name}: 字段 "${f}" 存在于 types.ts 但缺失于 Rust`);

    if (missingInTs.length === 0 && missingInRust.length === 0) {
      const orderDiff = rustFields.some((f, i) => tsFields[i] !== f);
      if (orderDiff) {
        errors.push(
          `${name}: 字段顺序不一致\n    Rust: [${rustFields.join(', ')}]\n    TS:   [${tsFields.join(', ')}]`
        );
      }
    }
  }

  if (errors.length > 0) {
    console.error(`[contract-check] 失败：Rust 模型与前端类型定义不一致（共 ${errors.length} 处）`);
    for (const e of errors) console.error(`  - ${e}`);
    console.error('\n请同步更新 src-tauri/src/models.rs（或 src-tauri/src/db/）与 src/types.ts');
    process.exit(1);
  }

  console.log(
    `[contract-check] 通过：${sharedNames.length} 个共享结构字段一致（${sharedNames.join(', ')}）`
  );
}

main();
