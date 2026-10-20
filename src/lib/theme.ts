import type { ThemeMode } from "../types";

let systemQuery: MediaQueryList | undefined;
let systemListener: ((event: MediaQueryListEvent) => void) | undefined;

function clearSystemListener() {
  if (systemQuery && systemListener) {
    systemQuery.removeEventListener("change", systemListener);
  }
  systemQuery = undefined;
  systemListener = undefined;
}

export function applyTheme(mode: ThemeMode) {
  clearSystemListener();
  const root = document.documentElement;
  const applyResolvedTheme = (dark: boolean) => {
    root.dataset.theme = dark ? "dark" : "light";
    root.style.colorScheme = dark ? "dark" : "light";
  };

  if (mode == "system") {
    systemQuery = window.matchMedia("(prefers-color-scheme: dark)");
    applyResolvedTheme(systemQuery.matches);
    systemListener = (event) => applyResolvedTheme(event.matches);
    systemQuery.addEventListener("change", systemListener);
    return;
  }

  applyResolvedTheme(mode == "dark");
}

export function disposeTheme() {
  clearSystemListener();
}
