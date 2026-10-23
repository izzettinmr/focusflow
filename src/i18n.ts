import { useSyncExternalStore } from "react";
import english from "./locales/en.json";
import messages from "./locales/messages.en.json";
export type Language = "tr" | "en";
let language: Language = "tr";
const listeners = new Set<() => void>();
export const getLanguage = () => language;
export const locale = () => (language === "en" ? "en-GB" : "tr-TR");
export function setLanguage(value: Language) {
  document.documentElement.lang = value;
  if (value === language) return;
  language = value;
  listeners.forEach((listener) => listener());
}
export function useLanguage() {
  return useSyncExternalStore((listener) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, getLanguage);
}
export function t(key: string, ...args: (string | number)[]): string {
  const text =
    language === "en" ? ((english as Record<string, string>)[key] ?? key) : key;
  return text.replace(/\{(\d+)\}/g, (match, index) =>
    args[Number(index)] === undefined ? match : String(args[Number(index)]),
  );
}
// Only app-owned messages go through this function, never course names or paths.
export function message(text: string): string {
  if (language !== "en") return text;
  const exact = (english as Record<string, string>)[text];
  if (exact) return exact;
  let result = text;
  for (const [tr, en] of Object.entries(messages).sort(
    (a, b) => b[0].length - a[0].length,
  )) {
    result = result.split(tr).join(en);
  }
  return result;
}
