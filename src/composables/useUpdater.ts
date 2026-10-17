import { computed, ref } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";

export type UpdaterStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "downloaded"
  | "error";

interface UpdateInfo {
  version: string;
  notes: string;
  downloadUrl: string;
  fileName: string;
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
  const downloaded = ref(0);
  const total = ref(0);
  const savedPath = ref("");
  const error = ref("");
  let pendingUpdate: UpdateInfo | null = null;

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
    savedPath.value = "";
    downloaded.value = 0;
    total.value = 0;
    pendingUpdate = null;
    try {
      const update = await invoke<UpdateInfo | null>("check_update");
      if (update == null) {
        status.value = "up-to-date";
        return;
      }
      pendingUpdate = update;
      version.value = update.version;
      notes.value = update.notes;
      status.value = "available";
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason);
      status.value = "error";
    }
  }

  async function downloadUpdate() {
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
      const channel = new Channel<DownloadEvent>();
      channel.onmessage = (event) => {
        if (event.event == "Started") {
          total.value = event.contentLength ?? 0;
        } else if (event.event == "Progress") {
          downloaded.value += event.chunkLength;
        }
      };
      savedPath.value = await invoke<string>("download_update", {
        url: update.downloadUrl,
        fileName: update.fileName,
        onEvent: channel,
      });
      status.value = "downloaded";
      await invoke("reveal_path", { path: savedPath.value });
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
    savedPath,
    error,
    checkForUpdates,
    downloadUpdate,
  };
}
