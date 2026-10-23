import { invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core";

declare global {
  interface Window {
    FocusFlowAndroid?: {
      request: (id: string, command: string, args: string) => void;
    };
    __focusflowResult?: (
      id: string,
      response: { ok: boolean; value?: unknown; error?: string },
    ) => void;
  }
}
export const isAndroid = () => !!window.FocusFlowAndroid;
export const isNative = () => isAndroid() || isTauri();
let next = 0;
const pending = new Map<
  string,
  { resolve: (v: unknown) => void; reject: (e: Error) => void }
>();
window.__focusflowResult = (id, response) => {
  const request = pending.get(id);
  if (!request) return;
  pending.delete(id);
  if (response.ok) request.resolve(response.value);
  else request.reject(new Error(response.error || "İşlem tamamlanamadı."));
};
export function invoke<T = unknown>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (!isAndroid()) return tauriInvoke<T>(command, args);
  return new Promise<T>((resolve, reject) => {
    const id = String(++next);
    pending.set(id, { resolve: (v) => resolve(v as T), reject });
    try {
      window.FocusFlowAndroid!.request(id, command, JSON.stringify(args));
    } catch (e) {
      pending.delete(id);
      reject(e);
    }
  });
}
