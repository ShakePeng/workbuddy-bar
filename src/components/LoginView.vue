<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, SettingsView } from "../types";

const baseUrl = ref("http://47.99.55.3:7864");
const username = ref("");
const password = ref("");
const showPassword = ref(false);
const loading = ref(false);
const error = ref("");
const initialized = ref(false);

const isHttp = computed(() => baseUrl.value.trim().toLowerCase().startsWith("http://"));

async function load() {
  try {
    const settings = await invoke<SettingsView>("get_settings");
    baseUrl.value = settings.baseUrl;
    username.value = settings.username;
    password.value = settings.password;
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
      username: username.value,
      password: password.value,
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
        <span>账号</span>
        <input v-model="username" type="text" autocomplete="username" placeholder="请输入管理端账号" required />
      </label>
      <label>
        <span>密码</span>
        <div class="password-field">
          <input
            v-model="password"
            :type="showPassword ? 'text' : 'password'"
            autocomplete="current-password"
            placeholder="请输入管理端密码"
            required
          />
          <button type="button" class="text-button" @click="showPassword = !showPassword">
            {{ showPassword ? "隐藏" : "显示" }}
          </button>
        </div>
      </label>

      <p v-if="isHttp" class="security-note">
        当前地址使用明文 HTTP。账号密码会保存在本机配置文件中，并直接发送到该地址。
      </p>
      <p v-if="error" class="form-error">{{ error }}</p>

      <button class="primary-button login-button" type="submit" :disabled="loading || !initialized">
        {{ loading ? "正在登录…" : "登录并启动监测" }}
      </button>
      <p class="login-hint">登录成功后窗口会收起，可从菜单栏图标随时查看状态。</p>
    </form>
  </main>
</template>
