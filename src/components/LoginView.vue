<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, SettingsView } from "../types";

const baseUrl = ref("");
const token = ref("");
const showToken = ref(false);
const loading = ref(false);
const error = ref("");
const initialized = ref(false);

const isHttp = computed(() => baseUrl.value.trim().toLowerCase().startsWith("http://"));

async function load() {
  try {
    const settings = await invoke<SettingsView>("get_settings");
    baseUrl.value = settings.baseUrl;
    token.value = settings.token;
  } catch (reason) {
    error.value = String(reason);
  } finally {
    initialized.value = true;
  }
}

async function submit() {
  error.value = "";
  loading.value = true;
  try {
    await invoke<AppSnapshot>("login", {
      baseUrl: baseUrl.value,
      token: token.value,
    });
  } catch (reason) {
    error.value = String(reason);
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <main class="login-page">
    <div class="brand-lockup">
      <img src="/workbuddy.png" alt="WorkBuddy" class="brand-icon" />
      <div>
        <h1>WorkBuddyBar</h1>
        <p>管理端只读状态栏</p>
      </div>
    </div>

    <form class="login-card" @submit.prevent="submit">
      <label>
        <span>管理端地址</span>
        <input v-model="baseUrl" type="url" autocomplete="url" placeholder="http://host:port" required />
      </label>
      <label>
        <span>访问令牌</span>
        <div class="password-field">
          <input
            v-model="token"
            :type="showToken ? 'text' : 'password'"
            autocomplete="off"
            placeholder="粘贴管理端创建的访问令牌（wbt_…）"
            required
          />
          <button type="button" class="text-button" @click="showToken = !showToken">
            {{ showToken ? "隐藏" : "显示" }}
          </button>
        </div>
        <small>访问令牌由管理员在管理端创建，创建后请立即复制保存。</small>
      </label>

      <p v-if="isHttp" class="security-note">
        当前地址使用明文 HTTP。访问令牌会保存在本机配置文件中，并直接发送到该地址。
      </p>
      <p v-if="error" class="form-error">{{ error }}</p>

      <button class="primary-button login-button" type="submit" :disabled="loading || !initialized">
        {{ loading ? "正在验证…" : "保存并启动监测" }}
      </button>
      <p class="login-hint">验证通过后窗口会收起，可从菜单栏图标随时查看状态。</p>
    </form>
  </main>
</template>
