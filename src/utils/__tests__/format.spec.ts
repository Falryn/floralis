import { describe, it, expect, vi } from "vitest";

vi.mock("../invoke", () => ({ invoke: vi.fn(() => Promise.resolve()) }));

import { formatPlayTime, formatDate, escapeHtml, highlightText, formatBytes } from "../format";

// 桩翻译函数：返回 key 与参数，便于断言分支走向
const t = (key: string, params?: Record<string, unknown>) =>
  `${key}|${JSON.stringify(params ?? {})}`;

describe("formatPlayTime", () => {
  it("uses seconds branch below one minute", () => {
    expect(formatPlayTime(0, t)).toBe('game.seconds|{"n":0}');
    expect(formatPlayTime(59, t)).toBe('game.seconds|{"n":59}');
  });

  it("uses minutes-only branch below one hour", () => {
    expect(formatPlayTime(60, t)).toBe('game.minutesOnly|{"m":1}');
    expect(formatPlayTime(3599, t)).toBe('game.minutesOnly|{"m":59}');
  });

  it("uses hours+minutes branch from one hour on", () => {
    expect(formatPlayTime(3600, t)).toBe('game.hoursMinutes|{"h":1,"m":0}');
    expect(formatPlayTime(3661, t)).toBe('game.hoursMinutes|{"h":1,"m":1}');
  });

  it("uses short variants when short=true", () => {
    expect(formatPlayTime(120, t, "game", true)).toBe('game.minutesShort|{"m":2}');
    expect(formatPlayTime(3661, t, "game", true)).toBe('game.hoursMinutesShort|{"h":1,"m":1}');
  });

  it("respects custom i18n prefix", () => {
    expect(formatPlayTime(30, t, "stats")).toBe('stats.seconds|{"n":30}');
    expect(formatPlayTime(7200, t, "stats")).toBe('stats.hoursMinutes|{"h":2,"m":0}');
  });
});

describe("formatDate", () => {
  it("returns never-key for null/empty", () => {
    expect(formatDate(null, t)).toBe("game.never|{}");
    expect(formatDate("", t)).toBe("game.never|{}");
  });

  it("shows date only by default", () => {
    expect(formatDate("2026-08-30T12:34:56", t)).toBe("2026-08-30");
  });

  it("shows date and time when showTime=true", () => {
    expect(formatDate("2026-08-30T12:34:56", t, true)).toBe("2026-08-30 12:34");
  });
});

describe("formatBytes", () => {
  it("returns 0 B for zero/negative/non-finite input", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(-5)).toBe("0 B");
    expect(formatBytes(NaN)).toBe("0 B");
  });

  it("keeps bytes as integers", () => {
    expect(formatBytes(1)).toBe("1 B");
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("converts to higher units with one decimal", () => {
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1048576)).toBe("1 MB");
    expect(formatBytes(1073741824)).toBe("1 GB");
  });

  it("drops decimals for large values", () => {
    expect(formatBytes(150 * 1024 * 1024)).toBe("150 MB");
  });
});

describe("escapeHtml", () => {
  it("escapes &, < and >", () => {
    expect(escapeHtml("<a & b>")).toBe("&lt;a &amp; b&gt;");
  });

  it("leaves plain text untouched", () => {
    expect(escapeHtml("hello")).toBe("hello");
  });
});

describe("highlightText", () => {
  it("returns escaped text when keyword is empty", () => {
    expect(highlightText("<b>", "")).toBe("&lt;b&gt;");
    expect(highlightText("abc", "   ")).toBe("abc");
  });

  it("wraps case-insensitive matches in <mark>", () => {
    const out = highlightText("Hello World", "world");
    expect(out).toContain("<mark");
    expect(out).toContain(">World</mark>");
  });

  it("treats regex special characters in keyword as literals", () => {
    const out = highlightText("a.b and axb", "a.b");
    expect(out).toContain(">a.b</mark> and axb");
    expect(out).not.toContain(">axb</mark>");
  });

  it("does not throw on keywords made only of special chars", () => {
    expect(() => highlightText("x (y) z", "(y)")).not.toThrow();
    expect(highlightText("x (y) z", "(y)")).toContain(">(y)</mark>");
  });

  it("escapes text HTML before wrapping matches", () => {
    const out = highlightText("<x>y", "y");
    expect(out).toBe('&lt;x&gt;<mark class="bg-yellow-200/70 text-inherit rounded-sm px-0.5">y</mark>');
  });
});
