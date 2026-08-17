/**
 * Mod 相关共享常量和工具函数
 */

type TFunction = (key: string, params?: Record<string, unknown>) => string;

/**
 * Mod 分类列表（标签通过 i18n 的 mod.cat.* 获取）
 */
export const MOD_CATEGORIES = [
  'textures', 'models', 'scripts', 'animations', 'audio', 'ui', 'gameplay',
  'cheats', 'saves', 'weapons', 'armor', 'characters', 'environment',
  'graphics', 'bugfixes', 'overhauls', 'quests', 'magic', 'npcs', 'utilities', 'misc',
] as const;

/**
 * 获取分类的本地化标签
 * @param category 分类 ID
 * @param t i18n 翻译函数
 */
export function categoryLabel(category: string, t: TFunction): string {
  if (!category) return "";
  const key = `mod.cat.${category}`;
  const translated = t(key);
  return translated !== key ? translated : category;
}
