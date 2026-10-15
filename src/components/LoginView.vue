<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppSnapshot, AuthMode, SettingsView } from "../types";

const baseUrl = ref("");
const authMode = ref<AuthMode>("token");
const token = ref("");
const username = ref("");
const password = ref("");
const showToken = ref(false);
const showPassword = ref(false);
const loading = ref(false);
const error = ref("");
const initialized = ref(false);
let unlisten: UnlistenFn | undefined;

const isHttp = computed(() => baseUrl.value.trim().toLowerCase().startsWith("http://"));
const isPasswordMode = computed(() => authMode.value === "password");

async function load() {
  try {
    const settings = await invoke<SettingsView>("get_settings");
    baseUrl.value = settings.baseUrl;
    authMode.value = settings.authMode;
    token.value = settings.token;
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
      authMode: authMode.value,
      token: token.value,
      username: username.value,
      password: password.value,
    });
  } catch (reason) {
    error.value = String(reason);
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  await load();
  // 会话失效又自动重登失败时，后端会把登录窗口叫回来；
  // 这里把失败原因带出来，避免用户回到登录页却看不到为什么。
  unlisten = await listen<AppSnapshot>("snapshot-changed", (event) => {
    const snapshot = event.payload;
    if (snapshot.error && snapshot.authState != "ok") {
      error.value = snapshot.error;
    }
  });
});

onUnmounted(() => unlisten?.());
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
      <div class="auth-mode-switch" role="tablist" aria-label="登录方式">
        <button
          type="button"
          role="tab"
          :class="{ active: !isPasswordMode }"
          :aria-selected="!isPasswordMode"
          @click="authMode = 'token'"
        >
          访问令牌
        </button>
        <button
          type="button"
          role="tab"
          :class="{ active: isPasswordMode }"
          :aria-selected="isPasswordMode"
          @click="authMode = 'password'"
        >
          账户密码
        </button>
      </div>

      <label>
        <span>管理端地址</span>
        <input v-model="baseUrl" type="url" autocomplete="url" placeholder="http://host:port" required />
      </label>

      <label v-if="!isPasswordMode">
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

      <template v-else>
        <label>
          <span>用户名</span>
          <input v-model="username" type="text" autocomplete="username" placeholder="管理端账号" required />
        </label>
        <label>
          <span>密码</span>
          <div class="password-field">
            <input
              v-model="password"
              :type="showPassword ? 'text' : 'password'"
              autocomplete="current-password"
              placeholder="管理端密码"
              required
            />
            <button type="button" class="text-button" @click="showPassword = !showPassword">
              {{ showPassword ? "隐藏" : "显示" }}
            </button>
          </div>
          <small>账号密码保存在本机配置文件中，会话过期时自动重新登录。</small>
        </label>
      </template>

      <p v-if="isHttp" class="security-note">
        {{
          isPasswordMode
            ? "当前地址使用明文 HTTP，账号密码会保存在本机配置文件中，并直接发送到该地址。"
            : "当前地址使用明文 HTTP。访问令牌会保存在本机配置文件中，并直接发送到该地址。"
        }}
      </p>
      <p v-if="error" class="form-error">{{ error }}</p>

      <button class="primary-button login-button" type="submit" :disabled="loading || !initialized">
        {{ loading ? "正在验证…" : "保存并启动监测" }}
      </button>
      <p class="login-hint">验证通过后窗口会收起，可从菜单栏图标随时查看状态。</p>
    </form>
  </main>
</template>
