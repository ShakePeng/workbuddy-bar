<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { UPDATE_WINDOW_OPENED_EVENT, useUpdater } from "../composables/useUpdater";

interface ReleaseLine {
  kind: "heading" | "item" | "paragraph";
  text: string;
}

const currentVersion = ref("");
const loading = ref(true);
let unlisten: UnlistenFn | undefined;
let loadToken = 0;

const {
  status,
  version,
  notes,
  selfUpdate,
  progressPercent,
  progressLabel,
  savedPath,
  error,
  loadPendingUpdate,
  downloadUpdate,
} = useUpdater();

const busy = computed(() => status.value == "downloading" || status.value == "installing");
const canRetry = computed(() => version.value != "");
const actionLabel = computed(() => (selfUpdate.value ? "安装更新" : "下载安装包"));
const headline = computed(() => {
  if (status.value == "available") return `WorkBuddyBar v${version.value} 已发布`;
  if (status.value == "downloading" || status.value == "installing") {
    return "正在更新 WorkBuddyBar";
  }
  if (status.value == "downloaded") return "安装包已下载";
  return "检查更新";
});
const description = computed(() => {
  if (status.value == "available") {
    return `您当前版本是 v${currentVersion.value}，新版本已经可以安装。`;
  }
  if (status.value == "downloading") {
    return `正在下载 v${version.value}，完成后会继续安装或打开安装包所在位置。`;
  }
  if (status.value == "installing") return "安装完成后应用会自动重启。";
  if (status.value == "downloaded") return "请运行已下载的安装包完成更新。";
  if (status.value == "error") return "暂时无法完成更新，请检查网络后重试。";
  return "没有可用的更新信息，请关闭窗口后重新检查。";
});
const releaseLines = computed(() => parseReleaseNotes(notes.value));

function cleanInlineMarkdown(text: string): string {
  return text
    .replace(/\*\*(.*?)\*\*/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .trim();
}

function parseReleaseNotes(raw: string): ReleaseLine[] {
  const lines: ReleaseLine[] = [];
  for (const source of raw.split("\n")) {
    const line = source.trim();
    if (!line) continue;
    if (line.startsWith("#")) {
      lines.push({
        kind: "heading",
        text: cleanInlineMarkdown(line.replace(/^#+\s*/, "")),
      });
      continue;
    }
    if (line.startsWith("- ") || line.startsWith("* ")) {
      lines.push({
        kind: "item",
        text: cleanInlineMarkdown(line.slice(2)),
      });
      continue;
    }
    lines.push({ kind: "paragraph", text: cleanInlineMarkdown(line) });
  }
  return lines;
}

async function reload() {
  const token = ++loadToken;
  loading.value = true;
  await loadPendingUpdate();
  if (token == loadToken) loading.value = false;
}

async function closeWindow() {
  await invoke("hide_update_window");
}

onMounted(async () => {
  currentVersion.value = await getVersion().catch(() => "未知");
  unlisten = await listen(UPDATE_WINDOW_OPENED_EVENT, () => {
    if (!busy.value) void reload();
  });
  await reload();
});

onUnmounted(() => {
  loadToken += 1;
  unlisten?.();
});
</script>

<template>
  <main class="update-page">
    <section class="update-dialog">
      <header class="update-dialog-header">
        <img src="/workbuddy.png" alt="" />
        <div>
          <small>软件更新</small>
          <h1>{{ headline }}</h1>
          <p>{{ description }}</p>
        </div>
      </header>

      <div v-if="loading" class="update-loading">
        <span class="update-spinner"></span>
        <p>正在读取更新信息…</p>
      </div>

      <template v-else-if="status == 'available'">
        <div class="release-notes">
          <div
            v-for="(line, index) in releaseLines"
            :key="`${index}-${line.text}`"
            :class="['release-line', line.kind]"
          >
            <span v-if="line.kind == 'item'"></span>
            <p>{{ line.text }}</p>
          </div>
          <p v-if="!releaseLines.length" class="release-empty">本次更新没有附加说明。</p>
        </div>
        <footer class="update-dialog-actions">
          <button type="button" class="secondary-button" @click="closeWindow">
            稍后提醒我
          </button>
          <button type="button" class="primary-button update-install-button" @click="downloadUpdate">
            {{ actionLabel }}
          </button>
        </footer>
      </template>

      <template v-else-if="status == 'downloading' || status == 'installing'">
        <div class="update-progress-panel">
          <div class="update-progress-title">
            <span>{{ status == "installing" ? "正在准备安装…" : "正在下载更新…" }}</span>
            <b>{{ progressPercent }}%</b>
          </div>
          <div class="progress-track update-progress-track">
            <i :style="{ width: `${progressPercent}%` }"></i>
          </div>
          <small>{{ progressLabel }}</small>
        </div>
        <footer class="update-dialog-actions">
          <button type="button" class="secondary-button" disabled>稍后提醒我</button>
        </footer>
      </template>

      <template v-else-if="status == 'downloaded'">
        <div class="update-result-panel success">
          <b>安装包已下载</b>
          <p>{{ savedPath }}</p>
          <small>已打开安装包所在位置。运行安装包完成更新后，请重新打开应用。</small>
        </div>
        <footer class="update-dialog-actions">
          <button type="button" class="primary-button" @click="closeWindow">关闭</button>
        </footer>
      </template>

      <template v-else>
        <div class="update-result-panel error">
          <b>{{ error || "更新信息已失效" }}</b>
          <p>请关闭窗口后重新点击「检查更新」。</p>
        </div>
        <footer class="update-dialog-actions">
          <button type="button" class="secondary-button" @click="closeWindow">关闭</button>
          <button
            v-if="canRetry"
            type="button"
            class="primary-button"
            @click="downloadUpdate"
          >
            重试
          </button>
        </footer>
      </template>
    </section>
  </main>
</template>
