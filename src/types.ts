import { t, locale, type Language } from "./i18n";
export type Category = {
  id: number;
  name: string;
  color: string;
  archived: boolean;
};
export type Session = {
  id: number;
  mode: "pomodoro" | "stopwatch";
  kind: "work" | "break";
  category_id: number;
  started_at: number;
  ended_at: number | null;
  status: "running" | "paused" | "completed" | "early";
  target_ms: number;
  elapsed_ms: number;
};
export type Settings = {
  language: Language;
  sound_enabled: boolean;
  sound_volume: number;
  work_minutes: number;
  short_minutes: number;
  long_minutes: number;
  goal_minutes: number;
  long_every: number;
};
export type Snapshot = {
  categories: Category[];
  sessions: Session[];
  days: { day: string; session_id: number; duration_ms: number }[];
  settings: Settings;
  recovery: boolean;
  notice: string | null;
};
export const dayKey = (d = new Date()) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
export function duration(ms: number) {
  const m = Math.floor(ms / 60000);
  return m >= 60
    ? `${Math.floor(m / 60)} ${t("sa")} ${m % 60} ${t("dk")}`
    : `${m} ${t("dk")}`;
}
export function clock(ms: number) {
  const s = Math.floor(Math.max(0, ms) / 1000);
  return `${Math.floor(s / 3600) ? String(Math.floor(s / 3600)).padStart(2, "0") + ":" : ""}${String(Math.floor(s / 60) % 60).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}
export const dateTime = (n: number) =>
  new Date(n).toLocaleString(locale(), {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
export const localInput = (n: number) => {
  const d = new Date(n);
  return new Date(n - d.getTimezoneOffset() * 60000).toISOString().slice(0, 19);
};
