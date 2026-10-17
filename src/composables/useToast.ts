import { ref } from "vue";

export type ToastKind = "error" | "success";

export function useToast() {
  const message = ref("");
  const kind = ref<ToastKind>("error");
  let timer = 0;

  function show(text: string, toastKind: ToastKind = "error") {
    if (!text) return;
    message.value = text;
    kind.value = toastKind;
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      message.value = "";
    }, 4000);
  }

  return { message, kind, show };
}
