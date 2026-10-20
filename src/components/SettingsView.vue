<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import type { AppSnapshot, AuthMode, SettingsInput, SettingsView, ThemeMode } from "../types";
import { useToast } from "../composables/useToast";
import { useUpdater } from "../composables/useUpdater";
import { applyTheme } from "../lib/theme";

const loading = ref(true);
const saving = ref(false);
const saved = ref(false);
const error = ref("");
const settings = ref<SettingsView | null>(null);
const authMode = ref<AuthMode>("token");
const form = ref<SettingsInput>({
  baseUrl: "",
  token: "",
  username: "",
  password: "",
  refreshIntervalSec: 120,
  lowCreditThreshold: 1000,
  launchAtLogin: false,
  themeMode: "system",
  singleColumn: false,
  showCreditOverview: true,
  showUsage: true,
  showAccounts: true,
  showKeys: true,
});
const showToken = ref(false);
const showPassword = ref(false);
const currentVersion = ref("");
const themeOptions: Array<{ value: ThemeMode; label: string }> = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
];

const {
  status: updateStatus,
  error: updateError,
  checkForUpdates,
} = useUpdater();
const { message: toastMessage, kind: toastKind, show: showToast } = useToast();

watch(error, (text) => {
  if (text) showToast(text);
});
watch(saved, (value) => {
  if (value) showToast("设置已保存并刷新数据", "success");
});
watch(updateError, (text) => {
  if (text) showToast(text);
});
watch(updateStatus, (status) => {
  if (status == "up-to-date") showToast("已是最新版本", "success");
});

const isHttp = computed(() => form.value.baseUrl.trim().toLowerCase().startsWith("http://"));
const isPasswordMode = computed(() => authMode.value === "password");

async function load() {
  loading.value = true;
  try {
    const value = await invoke<SettingsView>("get_settings");
    settings.value = value;
    authMode.value = value.authMode;
    applyTheme(value.themeMode);
    form.value = {
      baseUrl: value.baseUrl,
      token: value.token,
      username: value.username,
      password: value.password,
      refreshIntervalSec: value.refreshIntervalSec,
      lowCreditThreshold: value.lowCreditThreshold,
      launchAtLogin: value.launchAtLogin,
      themeMode: value.themeMode,
      singleColumn: value.singleColumn,
      showCreditOverview: value.showCreditOverview,
      showUsage: value.showUsage,
      showAccounts: value.showAccounts,
      showKeys: value.showKeys,
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
    await invoke<AppSnapshot>("save_settings", {
      settings: form.value,
    });
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

onMounted(async () => {
  await load();
  currentVersion.value = await getVersion().catch(() => "未知");
});
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
    <div class="settings-scroll">
    <div class="settings-grid">
      <section class="settings-section">
        <h2>连接</h2>
        <div class="mode-row">
          <span>
            登录方式
            <small>更换方式请先退出登录</small>
          </span>
          <b class="mode-badge">{{ isPasswordMode ? "账户密码" : "访问令牌" }}</b>
        </div>
        <label>
          <span>管理端地址</span>
          <input v-model="form.baseUrl" type="url" required />
        </label>
        <label v-if="!isPasswordMode">
          <span>访问令牌</span>
          <div class="password-field">
            <input
              v-model="form.token"
              :type="showToken ? 'text' : 'password'"
              autocomplete="off"
              placeholder="wbt_…"
              required
            />
            <button type="button" class="text-button" @click="showToken = !showToken">
              {{ showToken ? "隐藏" : "显示" }}
            </button>
          </div>
          <small>访问令牌由管理员在管理端创建。</small>
        </label>
        <template v-else>
          <label>
            <span>用户名</span>
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
            <small>会话过期时会用保存的账号密码自动重新登录。</small>
          </label>
        </template>
        <p v-if="isHttp" class="security-note">
          {{
            isPasswordMode
              ? "该地址使用明文 HTTP，账号密码会写入本机配置文件并在网络上明文传输。"
              : "该地址使用明文 HTTP，访问令牌会写入本机配置文件并在网络上明文传输。"
          }}
        </p>
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

      <section class="settings-section wide">
        <h2>外观与首页</h2>
        <div class="theme-setting">
          <span>
            <b>配色</b>
            <small>默认跟随系统外观</small>
          </span>
          <div class="auth-mode-switch theme-options" role="radiogroup" aria-label="配色">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              type="button"
              role="radio"
              :aria-checked="form.themeMode == option.value"
              :class="{ active: form.themeMode == option.value }"
              @click="form.themeMode = option.value"
            >
              {{ option.label }}
            </button>
          </div>
        </div>
        <label class="switch-row">
          <span>
            <b>单列展示</b>
            <small>默认关闭；关闭时保持当前两列布局</small>
          </span>
          <input v-model="form.singleColumn" type="checkbox" />
        </label>
        <div class="module-setting">
          <span>
            <b>展示模块</b>
            <small>取消勾选后不在首页展示，默认全部勾选</small>
          </span>
          <div class="module-options">
            <label>
              <input v-model="form.showCreditOverview" type="checkbox" />
              <span>积分概览</span>
            </label>
            <label>
              <input v-model="form.showUsage" type="checkbox" />
              <span>今日用量</span>
            </label>
            <label>
              <input v-model="form.showAccounts" type="checkbox" />
              <span>账号明细</span>
            </label>
            <label>
              <input v-model="form.showKeys" type="checkbox" />
              <span>密钥用量</span>
            </label>
          </div>
        </div>
      </section>

      <section class="settings-section">
        <h2>软件更新</h2>
        <div class="update-row">
          <span>当前版本</span>
          <b>v{{ currentVersion }}</b>
        </div>
        <div class="update-actions">
          <button
            type="button"
            class="secondary-button"
            :disabled="updateStatus == 'checking'"
            @click="checkForUpdates"
          >
            {{ updateStatus == "checking" ? "检查中…" : "检查更新" }}
          </button>
        </div>
      </section>

      <section class="settings-section compact">
        <h2>本地配置</h2>
        <code>{{ settings?.configPath }}</code>
        <small>配置文件权限限制为 0600，令牌与账号密码不会写入日志。</small>
      </section>
    </div>
    </div>

      <footer class="settings-actions">
        <button type="button" class="secondary-button" @click="openManager">打开管理端</button>
        <button type="button" class="secondary-button danger" @click="logout">退出登录</button>
        <button type="submit" class="primary-button" :disabled="saving">
          {{ saving ? "保存中…" : "保存设置" }}
        </button>
      </footer>
    </form>

    <div v-if="toastMessage" :class="['toast-message', toastKind]">{{ toastMessage }}</div>
  </main>
</template>
