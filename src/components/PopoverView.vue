<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppSnapshot } from "../types";
import {
  accountStatusLabel,
  accountStatusTone,
  formatCredits,
  formatInteger,
  formatRelativeTime,
  formatReset,
  formatTokens,
  realmLabel,
} from "../lib/format";

const snapshot = ref<AppSnapshot | null>(null);
const refreshing = ref(false);
const actionError = ref("");
const content = ref<HTMLElement | null>(null);
let unlisten: UnlistenFn | undefined;
let resizeTimer = 0;

const healthyCount = computed(
  () => snapshot.value?.accounts.filter((account) => account.status === "healthy").length ?? 0,
);
const totalLabel = computed(() => formatCredits(snapshot.value?.totalCredits));
const statusTone = computed(() => {
  const value = snapshot.value;
  if (!value || value.error) return "bad";
  if (!value.poolAvailable || value.stale || value.creditPartial) return "warn";
  return "good";
});
const statusText = computed(() => {
  const value = snapshot.value;
  if (!value) return "读取中";
  if (value.authState === "login_required") return "需要登录";
  if (value.error) return "连接异常";
  if (!value.poolAvailable) return "账号池不可达";
  if (value.stale) return "数据已过期";
  return "运行正常";
});
const lastUpdated = computed(() => formatRelativeTime(snapshot.value?.updatedAt));
const monthUsage = computed(() =>
  (snapshot.value?.usageDays ?? []).reduce(
    (total, day) => ({
      tokens: total.tokens + day.tokens,
      credit: total.credit + day.credit,
    }),
    { tokens: 0, credit: 0 },
  ),
);
const keyUsageTotal = computed(() =>
  (snapshot.value?.keyUsage ?? []).reduce(
    (total, key) => ({
      todayTokens: total.todayTokens + key.todayTokens,
      totalTokens: total.totalTokens + key.totalTokens,
    }),
    { todayTokens: 0, totalTokens: 0 },
  ),
);

async function resizeToContent() {
  await nextTick();
  window.clearTimeout(resizeTimer);
  resizeTimer = window.setTimeout(async () => {
    const element = content.value;
    if (!element) return;

    const height = Math.ceil(element.getBoundingClientRect().height + 28);
    await invoke("resize_popover", { height });
  }, 0);
}

async function load() {
  try {
    snapshot.value = await invoke<AppSnapshot>("load_state");
  } catch (reason) {
    actionError.value = String(reason);
  }
}

async function refresh() {
  refreshing.value = true;
  actionError.value = "";
  try {
    snapshot.value = await invoke<AppSnapshot>("refresh");
  } catch (reason) {
    actionError.value = String(reason);
  } finally {
    refreshing.value = false;
  }
}

async function openSettings() {
  await invoke("open_settings");
}

async function openManager() {
  try {
    await invoke("open_manager");
  } catch (reason) {
    actionError.value = String(reason);
  }
}

async function logout() {
  await invoke("logout");
}

onMounted(async () => {
  await load();
  await resizeToContent();
  unlisten = await listen<AppSnapshot>("snapshot-changed", (event) => {
    snapshot.value = event.payload;
  });
});

watch([snapshot, actionError, refreshing], resizeToContent, { deep: true });

onUnmounted(() => {
  window.clearTimeout(resizeTimer);
  unlisten?.();
});
</script>

<template>
  <main class="popover-page">
    <div ref="content" class="popover-content">
    <header class="popover-header" data-tauri-drag-region>
      <div class="mini-brand" data-tauri-drag-region>
        <img src="/workbuddy.png" alt="" />
        <div data-tauri-drag-region>
          <strong data-tauri-drag-region>WorkBuddy Manager</strong>
          <span :class="['status-dot', statusTone]" data-tauri-drag-region>{{ statusText }}</span>
        </div>
      </div>
      <div class="header-actions">
        <span v-if="snapshot?.username" class="current-user" :title="snapshot.username">{{ snapshot.username }}</span>
        <button class="icon-button" type="button" :disabled="refreshing" title="刷新" @click="refresh">
          <svg viewBox="0 0 24 24" :class="{ spinning: refreshing }"><path d="M20 11a8.1 8.1 0 0 0-14.9-3M4 5v4h4M4 13a8.1 8.1 0 0 0 14.9 3M20 19v-4h-4" /></svg>
        </button>
      </div>
    </header>

    <section class="hero-card" :class="statusTone">
      <div class="hero-top">
        <span>全池可用积分</span>
        <span class="freshness">{{ lastUpdated }}</span>
      </div>
      <div class="credit-number">{{ totalLabel }}</div>
      <div class="hero-meta">
        <span>{{ healthyCount }}/{{ snapshot?.accountTotal ?? 0 }} 个账号可用</span>
        <span v-if="snapshot?.creditPartial">部分余额未同步</span>
        <span v-else>已同步 {{ snapshot?.creditSyncedAccounts ?? 0 }} 个账户</span>
      </div>
    </section>

    <section class="section-card usage-card">
      <div class="section-title">
        <h2>今日用量</h2>
      </div>
      <div class="metric-grid">
        <div>
          <span>请求</span>
          <b>{{ formatInteger(snapshot?.today.todayRequests) }}</b>
        </div>
        <div>
          <span>Tokens</span>
          <b>{{ formatTokens(snapshot?.today.todayTokens) }}</b>
        </div>
        <div>
          <span>实际扣费</span>
          <b>{{ formatCredits(snapshot?.today.todayCredit) }}</b>
        </div>
      </div>
      <div class="week-line">
        <span>近 7 天</span>
        <b>{{ formatCredits(snapshot?.today.weekCredit) }} 积分</b>
        <span>{{ formatTokens(snapshot?.today.weekTokens) }} Tokens</span>
      </div>
      <div class="week-line month-line">
        <span>近 30 天</span>
        <b>{{ formatCredits(monthUsage.credit) }} 积分</b>
        <span>{{ formatTokens(monthUsage.tokens) }} Tokens</span>
      </div>
    </section>

    <section v-if="snapshot" class="section-card keys-card">
      <div class="section-title">
        <h2>密钥用量</h2>
        <span>{{ snapshot?.keyUsage.length ?? 0 }} 个</span>
      </div>
      <template v-if="snapshot?.keyUsage.length">
        <div class="key-usage-head">
          <span>密钥</span>
          <span>今日</span>
          <span>累计</span>
        </div>
        <div v-if="(snapshot?.keyUsage.length ?? 0) > 1" class="key-usage-total">
          <span>全部密钥</span>
          <b>{{ formatTokens(keyUsageTotal.todayTokens) }}</b>
          <b>{{ formatTokens(keyUsageTotal.totalTokens) }}</b>
        </div>
        <div class="key-usage-list">
          <div
            v-for="key in snapshot.keyUsage"
            :key="key.id"
            :class="['key-usage-row', { disabled: !key.enabled }]"
          >
            <div class="key-usage-main">
              <span>{{ key.name }}</span>
            </div>
            <b>{{ formatTokens(key.todayTokens) }}</b>
            <b>{{ formatTokens(key.totalTokens) }}</b>
          </div>
        </div>
      </template>
      <div v-else class="empty-state">暂无密钥用量</div>
    </section>

    <section class="section-card accounts-card">
      <div class="section-title">
        <h2>账号明细</h2>
        <span>{{ snapshot?.accounts.length ?? 0 }} 个</span>
      </div>
      <div v-if="snapshot?.accounts.length" class="account-list">
        <div v-for="account in snapshot.accounts" :key="account.uid || account.file" class="account-row">
          <div class="account-main">
            <div class="account-name">
              <span>{{ account.nickname }}</span>
              <i :class="account.realm === 'global' ? 'realm global' : 'realm'">{{ realmLabel(account.realm) }}</i>
            </div>
            <small>
              {{ accountStatusLabel(account.status) }}
              <template v-if="account.expiresAt"> · 到期 {{ formatReset(account.expiresAt) }}</template>
            </small>
          </div>
          <div class="account-credit">
            <b>{{ formatCredits(account.credits) }}</b>
            <i :class="['account-tone', accountStatusTone(account.status)]"></i>
          </div>
        </div>
      </div>
      <div v-else class="empty-state">暂未读取到账号数据</div>
    </section>

    <p v-if="snapshot?.error" class="inline-error">{{ snapshot.error }}</p>
    <p v-if="actionError" class="inline-error">{{ actionError }}</p>

    <footer class="popover-footer">
      <button type="button" @click="openManager">
        <svg viewBox="0 0 24 24"><path d="M14 4h6v6M20 4l-9 9M19 13v6a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h6" /></svg>
        管理端
      </button>
      <button type="button" @click="openSettings">
        <svg viewBox="0 0 24 24"><path d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z" /><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-1.42 1.42-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.04 1.56V20h-2v-.08A1.7 1.7 0 0 0 12 18.36a1.7 1.7 0 0 0-1.88.34l-.06.06-1.42-1.42.06-.06A1.7 1.7 0 0 0 9.04 15a1.7 1.7 0 0 0-1.56-1.04H7v-2h.48A1.7 1.7 0 0 0 9.04 10a1.7 1.7 0 0 0-.34-1.88l-.06-.06 1.42-1.42.06.06A1.7 1.7 0 0 0 12 7.04 1.7 1.7 0 0 0 13.04 5.48V5h2v.48A1.7 1.7 0 0 0 16.6 7a1.7 1.7 0 0 0 1.88-.34l.06-.06 1.42 1.42-.06.06A1.7 1.7 0 0 0 19.56 10a1.7 1.7 0 0 0 1.56 1.04H21v2h-.48A1.7 1.7 0 0 0 19.4 15Z" /></svg>
        设置
      </button>
      <button type="button" class="danger" @click="logout">
        <svg viewBox="0 0 24 24"><path d="M10 17l5-5-5-5M15 12H3M21 4v16" /></svg>
        退出登录
      </button>
    </footer>
    </div>
  </main>
</template>
