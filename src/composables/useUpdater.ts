import { computed, ref } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";

export const UPDATE_WINDOW_OPENED_EVENT = "update-window-opened";

export type UpdaterStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "installing"
  | "downloaded"
  | "error";

export interface UpdateInfo {
  version: string;
  notes: string;
  downloadUrl: string;
  fileName: string;
  sha256: string | null;
  selfUpdate: boolean;
  manualDownloadUrl: string | null;
  manualFileName: string | null;
}

interface InstallResult {
  installed: boolean;
  manualRequired: boolean;
  savedPath: string | null;
}

type DownloadEvent =
  | { event: "Started"; contentLength?: number }
  | { event: "Progress"; chunkLength: number }
  | { event: "Finished" };

function formatBytes(value: number): string {
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  if (value >= 1024) return `${Math.round(value / 1024)} KB`;
  return `${value} B`;
}

export function useUpdater() {
  const status = ref<UpdaterStatus>("idle");
  const version = ref("");
  const notes = ref("");
  const selfUpdate = ref(false);
  const downloaded = ref(0);
  const total = ref(0);
  const savedPath = ref("");
  const error = ref("");
  let pendingUpdate: UpdateInfo | null = null;

  const progressPercent = computed(() => {
    if (status.value == "installing") return 100;
    if (total.value > 0) {
      return Math.min(100, Math.round((downloaded.value / total.value) * 100));
    }
    return 0;
  });
  const progressLabel = computed(() => {
    if (status.value == "installing") return "正在准备安装并重启…";
    if (total.value > 0) return `${formatBytes(downloaded.value)} / ${formatBytes(total.value)}`;
    if (downloaded.value > 0) return `已下载 ${formatBytes(downloaded.value)}`;
    return "正在连接更新服务器…";
  });

  function reset() {
    version.value = "";
    notes.value = "";
    selfUpdate.value = false;
    savedPath.value = "";
    downloaded.value = 0;
    total.value = 0;
    error.value = "";
    pendingUpdate = null;
  }

  function applyUpdate(update: UpdateInfo) {
    pendingUpdate = update;
    version.value = update.version;
    notes.value = update.notes;
    selfUpdate.value = update.selfUpdate;
  }

  async function checkForUpdates() {
    reset();
    status.value = "checking";
    try {
      const update = await invoke<UpdateInfo | null>("check_update");
      if (update == null) {
        status.value = "up-to-date";
        return;
      }
      applyUpdate(update);
      status.value = "available";
      await invoke("open_update_window");
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason);
      status.value = "error";
    }
  }

  async function loadPendingUpdate() {
    reset();
    try {
      const update = await invoke<UpdateInfo | null>("get_pending_update");
      if (update == null) {
        status.value = "idle";
        return false;
      }
      applyUpdate(update);
      status.value = "available";
      return true;
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason);
      status.value = "error";
      return false;
    }
  }

  async function downloadUpdate() {
    if (status.value == "downloading" || status.value == "installing") return;
    if (pendingUpdate == null) {
      error.value = "更新信息已失效，请关闭窗口后重新检查。";
      status.value = "error";
      return;
    }

    status.value = "downloading";
    error.value = "";
    savedPath.value = "";
    downloaded.value = 0;
    total.value = 0;
    try {
      const update = pendingUpdate;
      if (update.selfUpdate && update.sha256) {
        const channel = new Channel<DownloadEvent>();
        channel.onmessage = (event) => {
          if (event.event == "Started") {
            total.value = event.contentLength ?? 0;
          } else if (event.event == "Progress") {
            downloaded.value += event.chunkLength;
          } else if (event.event == "Finished") {
            status.value = "installing";
          }
        };
        const result = await invoke<InstallResult>("install_update", {
          url: update.downloadUrl,
          fileName: update.fileName,
          sha256: update.sha256,
          onEvent: channel,
        });
        if (result.installed) {
          status.value = "installing";
          return;
        }
      }
      if (update.manualDownloadUrl) {
        await downloadManual(
          update.manualDownloadUrl,
          update.manualFileName ?? update.fileName,
        );
        return;
      }
      await downloadManual(update.downloadUrl, update.fileName);
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason);
      status.value = "error";
    }
  }

  async function downloadManual(url: string, fileName: string) {
    const channel = new Channel<DownloadEvent>();
    channel.onmessage = (event) => {
      if (event.event == "Started") {
        total.value = event.contentLength ?? 0;
      } else if (event.event == "Progress") {
        downloaded.value += event.chunkLength;
      }
    };
    savedPath.value = await invoke<string>("download_update", {
      url,
      fileName,
      onEvent: channel,
    });
    status.value = "downloaded";
    await invoke("reveal_path", { path: savedPath.value });
  }

  return {
    status,
    version,
    notes,
    selfUpdate,
    progressPercent,
    progressLabel,
    savedPath,
    error,
    checkForUpdates,
    loadPendingUpdate,
    downloadUpdate,
  };
}
