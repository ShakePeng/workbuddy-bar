<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import LoginView from "./components/LoginView.vue";
import PopoverView from "./components/PopoverView.vue";
import SettingsView from "./components/SettingsView.vue";
import UpdateView from "./components/UpdateView.vue";
import { applyTheme, disposeTheme } from "./lib/theme";
import type { SettingsView as SettingsViewData } from "./types";

const view = new URLSearchParams(window.location.search).get("view") ?? "login";
let unlisten: UnlistenFn | undefined;

async function loadTheme() {
  try {
    const settings = await invoke<SettingsViewData>("get_settings");
    applyTheme(settings.themeMode);
  } catch {
    applyTheme("system");
  }
}

onMounted(async () => {
  await loadTheme();
  unlisten = await listen("settings-changed", () => {
    void loadTheme();
  });
});

onUnmounted(() => {
  unlisten?.();
  disposeTheme();
});
</script>

<template>
  <LoginView v-if="view === 'login'" />
  <SettingsView v-else-if="view === 'settings'" />
  <UpdateView v-else-if="view === 'update'" />
  <PopoverView v-else />
</template>
