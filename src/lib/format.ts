import type { AccountStatus } from "../types";

const numberFormatter = new Intl.NumberFormat("zh-CN", {
  maximumFractionDigits: 2,
});

const integerFormatter = new Intl.NumberFormat("zh-CN", {
  maximumFractionDigits: 0,
});

export function formatCredits(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  return numberFormatter.format(value);
}

export function formatInteger(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  return integerFormatter.format(value);
}

export function formatCompact(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  if (Math.abs(value) >= 100000) return `${(value / 1000).toFixed(1)}k`;
  return numberFormatter.format(value);
}

export function formatTokens(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  if (Math.abs(value) >= 1000000) return `${(value / 1000000).toFixed(2)}M`;
  if (Math.abs(value) >= 1000) return `${(value / 1000).toFixed(1)}k`;
  return integerFormatter.format(value);
}

export function formatRelativeTime(epochSeconds: number | null | undefined): string {
  if (!epochSeconds) return "尚未更新";
  const delta = Math.max(0, Math.round(Date.now() / 1000 - epochSeconds));
  if (delta < 15) return "刚刚更新";
  if (delta < 60) return `${delta} 秒前`;
  if (delta < 3600) return `${Math.floor(delta / 60)} 分钟前`;
  if (delta < 86400) return `${Math.floor(delta / 3600)} 小时前`;
  return `${Math.floor(delta / 86400)} 天前`;
}

export function formatReset(epochSeconds: number | null | undefined): string {
  if (!epochSeconds) return "未知";
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(epochSeconds * 1000));
}

export function accountStatusLabel(status: AccountStatus): string {
  return {
    healthy: "可用",
    cooling: "冷却中",
    disabled: "已停用",
    expired: "凭证过期",
    unavailable: "不可用",
    unknown: "状态未知",
  }[status];
}

export function accountStatusTone(status: AccountStatus): string {
  if (status == "healthy") return "good";
  if (status == "cooling" || status == "unknown") return "warn";
  return "bad";
}

export function realmLabel(realm: string): string {
  return realm == "global" ? "国际" : "国内";
}
