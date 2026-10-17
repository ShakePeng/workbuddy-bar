import { computed, ref } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdaterStatus = "idle" | "checking" | "up-to-date" | "available" | "downloading" | "error";

function formatBytes(value: number): string {
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  if (value >= 1024) return `${Math.round(value / 1024)} KB`;
  return `${value} B`;
}

export function useUpdater() {
  const status = ref<UpdaterStatus>("idle");
  const version = ref("");
  const notes = ref("");
  const downloaded = ref(0);
  const total = ref(0);
  const error = ref("");
  let pendingUpdate: Update | null = null;

  const progressPercent = computed(() => {
    if (total.value > 0) {
      return Math.min(100, Math.round((downloaded.value / total.value) * 100));
    }
    return 0;
  });
  const progressLabel = computed(() => {
    if (total.value > 0) return `${formatBytes(downloaded.value)} / ${formatBytes(total.value)}`;
    if (downloaded.value > 0) return `已下载 ${formatBytes(downloaded.value)}`;
    return "准备中…";
  });

  async function checkForUpdates() {
    status.value = "checking";
    error.value = "";
    version.value = "";
    notes.value = "";
    downloaded.value = 0;
    total.value = 0;
    pendingUpdate = null;
    try {
      const update = await check();
      if (update == null) {
        status.value = "up-to-date";
        return;
      }
      pendingUpdate = update;
      version.value = update.version;
      notes.value = update.body ?? "";
      status.value = "available";
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason);
      status.value = "error";
    }
  }

  async function installUpdate() {
    if (pendingUpdate == null) {
      status.value = "idle";
      return;
    }
    status.value = "downloading";
    error.value = "";
    downloaded.value = 0;
    total.value = 0;
    try {
      const update = pendingUpdate;
      await update.downloadAndInstall(
        (event) => {
          if (event.event == "Started" && event.data.contentLength) {
            total.value = event.data.contentLength;
          } else if (event.event == "Progress") {
            downloaded.value += event.data.chunkLength;
          }
        },
        { restartAfterInstall: true },
      );
      // Windows 上安装器启动后本进程直接退出，不会走到这里；macOS 需要手动重启新版本。
      await relaunch();
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason);
      status.value = "error";
    }
  }

  return {
    status,
    version,
    notes,
    progressPercent,
    progressLabel,
    error,
    checkForUpdates,
    installUpdate,
  };
}
