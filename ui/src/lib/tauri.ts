/** True when the UI runs inside the Tauri webview. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Closes the main window (File → Quit). No-op in Vite-only dev. */
export async function quitApp(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}
