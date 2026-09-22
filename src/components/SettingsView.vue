<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, SettingsView } from "../types";

const loading = ref(true);
const saving = ref(false);
const saved = ref(false);
const error = ref("");
const settings = ref<SettingsView | null>(null);
const form = ref({
  baseUrl: "http://47.99.55.3:7864",
  username: "",
  password: "",
  refreshIntervalSec: 60,
  lowCreditThreshold: 1000,
  launchAtLogin: false,
});
const showPassword = ref(false);

const isHttp = computed(() => form.value.baseUrl.trim().toLowerCase().startsWith("http://"));

async function load() {
  loading.value = true;
  try {
    const value = await invoke<SettingsView>("get_settings");
    settings.value = value;
    form.value = {
      baseUrl: value.baseUrl,
      username: value.username,
      password: value.password,
      refreshIntervalSec: value.refreshIntervalSec,
      lowCreditThreshold: value.lowCreditThreshold,
      launchAtLogin: value.launchAtLogin,
    };
  } catch (reason) {
    error.value = String(reason);
  } finally {
    loading.value = false;
  }
}

async function save() {
  saving.value = true;
  saved.value = false;
  error.value = "";
  try {
    await invoke<AppSnapshot>("save_settings", { settings: form.value });
    saved.value = true;
    await load();
    window.setTimeout(() => (saved.value = false), 1800);
  } catch (reason) {
    error.value = String(reason);
  } finally {
    saving.value = false;
  }
}

async function logout() {
  await invoke("logout");
}

async function openManager() {
  try {
    await invoke("open_manager");
  } catch (reason) {
    error.value = String(reason);
  }
}

onMounted(load);
</script>

<template>
  <main class="settings-page">
    <header class="settings-header">
      <div>
        <h1>WorkBuddyBar 设置</h1>
        <p>只读取管理端数据，不触发腾讯侧积分刷新或任务动作。</p>
      </div>
      <img src="/workbuddy.png" alt="" />
    </header>

    <form v-if="!loading" class="settings-form" @submit.prevent="save">
      <section class="settings-section">
        <h2>连接</h2>
        <label>
          <span>管理端地址</span>
          <input v-model="form.baseUrl" type="url" required />
        </label>
        <label>
          <span>账号</span>
          <input v-model="form.username" type="text" autocomplete="username" required />
        </label>
        <label>
          <span>密码</span>
          <div class="password-field">
            <input
              v-model="form.password"
              :type="showPassword ? 'text' : 'password'"
              autocomplete="current-password"
              required
            />
            <button type="button" class="text-button" @click="showPassword = !showPassword">
              {{ showPassword ? "隐藏" : "显示" }}
            </button>
          </div>
        </label>
        <p v-if="isHttp" class="security-note">该地址使用明文 HTTP，密码会写入本机配置文件并在网络上明文传输。</p>
      </section>

      <section class="settings-section">
        <h2>监测</h2>
        <label>
          <span>刷新间隔（秒）</span>
          <input v-model.number="form.refreshIntervalSec" type="number" min="30" max="600" step="10" required />
        </label>
        <label>
          <span>低积分界面阈值</span>
          <input v-model.number="form.lowCreditThreshold" type="number" min="0" step="100" required />
        </label>
        <label class="switch-row">
          <span>
            <b>登录系统后自动启动</b>
            <small>默认关闭</small>
          </span>
          <input v-model="form.launchAtLogin" type="checkbox" />
        </label>
      </section>

      <section class="settings-section compact">
        <h2>本地配置</h2>
        <code>{{ settings?.configPath }}</code>
        <small>配置文件权限限制为 0600，密码与会话 Cookie 不会写入日志。</small>
      </section>

      <p v-if="error" class="form-error">{{ error }}</p>
      <div v-if="saved" class="save-success">设置已保存并刷新数据</div>

      <footer class="settings-actions">
        <button type="button" class="secondary-button" @click="openManager">打开管理端</button>
        <button type="button" class="secondary-button danger" @click="logout">退出登录</button>
        <button type="submit" class="primary-button" :disabled="saving">
          {{ saving ? "保存中…" : "保存设置" }}
        </button>
      </footer>
    </form>
  </main>
</template>
