import { useEffect, useState, useRef, useMemo, type ReactNode } from "react";
import { invoke, isNative, isAndroid } from "./platform";
import { SyncPanel } from "./SyncPanel";
import { isEnabled, enable, disable } from "@tauri-apps/plugin-autostart";
import {
  Activity,
  ArrowRight,
  BarChart3,
  BookOpen,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Clock3,
  Coffee,
  Download,
  Flame,
  Focus,
  History,
  LayoutDashboard,
  Maximize2,
  Minimize2,
  MoreHorizontal,
  Pause,
  Play,
  Plus,
  RotateCcw,
  Settings2,
  Square,
  Target,
  Timer,
  Trash2,
  Upload,
  X,
  Pencil,
  PanelLeftClose,
  Power,
  CheckCircle2,
  ArrowUpRight,
} from "lucide-react";
import {
  Category,
  Session,
  Settings,
  Snapshot,
  dayKey,
  duration,
  clock,
  dateTime,
  localInput,
} from "./types";
type Page =
  "dashboard" | "timer" | "statistics" | "history" | "categories" | "settings";
const titles: Record<Page, [string, string]> = {
  dashboard: ["Genel bakış", "Küçük adımlar, büyük ilerlemeler."],
  timer: ["Odak zamanı", "Tek bir işe odaklan. Gerisi bekleyebilir."],
  statistics: ["İstatistikler", "Emeğini gör, ritmini keşfet."],
  history: ["Çalışma geçmişi", "Her oturum, hedeflerine bir adım daha."],
  categories: ["Derslerim", "Zamanını neye ayırdığını bil."],
  settings: ["Ayarlar", "Kendi çalışma ritmini oluştur."],
};
const modeName = (m: string) => (m === "pomodoro" ? "Pomodoro" : "Kronometre");
const statusName = (s: string) =>
  ({
    running: "Çalışıyor",
    paused: "Duraklatıldı",
    completed: "Tamamlandı",
    early: "Erken bitirildi",
  })[s] || s;
function Card({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return <section className={"card " + className}>{children}</section>;
}
function App() {
  const [data, setData] = useState<Snapshot>();
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [page, setPage] = useState<Page>("dashboard");
  const [mode, setMode] = useState<"pomodoro" | "stopwatch">("pomodoro");
  const [category, setCategory] = useState(1);
  const [kind, setKind] = useState("work");
  const [minutes, setMinutes] = useState(25);
  const [focus, setFocus] = useState(false);
  const [toast, setToast] = useState("");
  const [confirm, setConfirm] = useState<{
    text: string;
    resolve: (v: boolean) => void;
  }>();
  const [edit, setEdit] = useState<Session>();
  const initialized = useRef(false);
  const mounted = useRef(true);
  const refresh = async () => {
    const d = await invoke<Snapshot>("snapshot");
    if (mounted.current) {
      setData(d);
      setCategory((previous) =>
        d.categories.some((c) => c.id === previous && !c.archived)
          ? previous
          : d.categories.find((c) => !c.archived)?.id || 1,
      );
      if (!initialized.current) {
        initialized.current = true;
        setMinutes(d.settings.work_minutes);
        setCategory(d.categories.find((c) => !c.archived)?.id || 1);
        const a = d.sessions.find((s) =>
          ["running", "paused"].includes(s.status),
        );
        if (a) {
          setMode(a.mode);
          setKind(a.kind);
          setCategory(a.category_id);
          setPage("timer");
        }
      }
    }
  };
  useEffect(() => {
    mounted.current = true;
    if (!isNative()) {
      setError(
        "FocusFlow yerel bir masaüstü uygulamasıdır. Terminalde npm run desktop komutuyla açın.",
      );
      return;
    }
    let pending = false;
    const poll = async () => {
      if (pending || document.hidden) return;
      pending = true;
      try {
        await refresh();
      } catch (e) {
        setError(String(e));
      } finally {
        pending = false;
      }
    };
    void poll();
    const t = setInterval(poll, 1000);
    const visible = () => {
      if (!document.hidden) void poll();
    };
    document.addEventListener("visibilitychange", visible);
    return () => {
      mounted.current = false;
      clearInterval(t);
      document.removeEventListener("visibilitychange", visible);
    };
  }, []);
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(""), 4500);
    return () => clearTimeout(t);
  }, [toast]);
  const run = async (fn: () => Promise<unknown>, message?: string) => {
    if (busy) return;
    setBusy(true);
    try {
      await fn();
      await refresh();
      if (message) setToast(message);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const ask = (text: string) =>
    new Promise<boolean>((resolve) => setConfirm({ text, resolve }));
  const active = data?.sessions.find(
    (s) => s.status === "running" || s.status === "paused",
  );
  const toggle = () => {
    if (active)
      void run(() =>
        invoke("action", {
          action: active.status === "running" ? "pause" : "resume",
        }),
      );
    else
      void run(() =>
        invoke("start", { mode, kind, categoryId: category, minutes }),
      );
  };
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement;
      if (
        ["INPUT", "SELECT", "TEXTAREA", "BUTTON"].includes(el.tagName) ||
        edit ||
        confirm
      )
        return;
      if (e.code === "Space" && page === "timer") {
        e.preventDefault();
        if (!busy) toggle();
      }
      if (e.key.toLowerCase() === "f" && !e.ctrlKey && !e.metaKey) {
        setPage("timer");
        setFocus((f) => !f);
      }
      if (e.key === "Escape") setFocus(false);
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [active, mode, kind, minutes, category, busy, page, edit, confirm]);
  const today = dayKey();
  const liveCats = data?.categories.filter((c) => !c.archived) || [];
  const sessions = data?.sessions || [];
  const cat = (id: number) => data?.categories.find((c) => c.id === id);
  const sessionMap = useMemo(
    () => new Map(sessions.map((s) => [s.id, s])),
    [sessions],
  );
  const days = data?.days || [];
  const todayMs = days
    .filter((d) => d.day === today)
    .reduce((a, d) => a + d.duration_ms, 0);
  const goal = (data?.settings.goal_minutes || 240) * 60000;
  const percent = Math.min(100, Math.floor((todayMs / goal) * 100));
  const weekStart = new Date();
  weekStart.setDate(weekStart.getDate() - ((weekStart.getDay() + 6) % 7));
  const weekKey = dayKey(weekStart);
  const monthKey = today.slice(0, 7);
  const weekMs = days
    .filter((d) => d.day >= weekKey && d.day <= today)
    .reduce((a, d) => a + d.duration_ms, 0);
  const monthMs = days
    .filter((d) => d.day.startsWith(monthKey))
    .reduce((a, d) => a + d.duration_ms, 0);
  const pomos = sessions.filter(
    (s) =>
      s.kind === "work" &&
      s.mode === "pomodoro" &&
      s.status === "completed" &&
      dayKey(new Date(s.ended_at!)) === today,
  ).length;
  const totalMode = (m: string) =>
    sessions
      .filter((s) => s.kind === "work" && s.mode === m)
      .reduce((a, s) => a + s.elapsed_ms, 0);
  const longest = sessions.reduce(
    (longest, s) =>
      s.kind === "work" ? Math.max(longest, s.elapsed_ms) : longest,
    0,
  );
  const streak = (() => {
    const worked = new Set(
      days.filter((d) => d.duration_ms >= 1000).map((d) => d.day),
    );
    const d = new Date();
    if (!worked.has(dayKey(d))) d.setDate(d.getDate() - 1);
    let n = 0;
    while (worked.has(dayKey(d))) {
      n++;
      d.setDate(d.getDate() - 1);
    }
    return n;
  })();
  const changeKind = (k: string) => {
    setKind(k);
    setMode("pomodoro");
    setMinutes(
      k === "work"
        ? data!.settings.work_minutes
        : k === "short"
          ? data!.settings.short_minutes
          : data!.settings.long_minutes,
    );
    if (k !== "work") setKind("break");
  };
  const finish = () =>
    void run(
      () => invoke("action", { action: "finish" }),
      "Oturum gerçek çalışma süresiyle kaydedildi.",
    );
  const cancel = async () => {
    if (
      await ask(
        "Aktif oturum ve bu oturuma ait çalışma süresi kalıcı olarak silinsin mi?",
      )
    )
      void run(
        () => invoke("action", { action: "cancel" }),
        "Oturum iptal edildi.",
      );
  };
  const exportData = (format: string) =>
    void run(async () => {
      const saved = await invoke<boolean>("export_file", { format });
      if (saved)
        setToast(
          format === "json" ? "Yedek kaydedildi." : "CSV dışa aktarıldı.",
        );
    });
  const restore = () =>
    void run(async () => {
      if (await invoke<boolean>("restore_file"))
        setToast("Yedek geri yüklendi.");
    });
  const timerPanel = (compact = false) => {
    const shown = active?.mode || mode;
    const isBreak = active ? active.kind === "break" : kind === "break";
    const elapsed = active?.elapsed_ms || 0;
    const target = active?.target_ms || minutes * 60000;
    const remaining =
      shown === "pomodoro" ? Math.max(0, target - elapsed) : elapsed;
    const progress = shown === "pomodoro" ? Math.min(1, elapsed / target) : 0;
    return (
      <Card className={"timer-card " + (compact ? "compact" : "")}>
        <div className="card-heading">
          <div className="eyebrow">
            <span
              className={
                "live-dot " + (active?.status === "running" ? "" : "quiet")
              }
            />
            {active?.status === "running"
              ? "ODAKLANIYORSUN"
              : isBreak
                ? "MOLA ZAMANI"
                : "KENDİNE BİR ALAN AÇ"}
          </div>
          <button
            className="icon-button"
            aria-label="Odak modunu aç veya kapat"
            onClick={() => {
              setPage("timer");
              setFocus(!focus);
            }}
          >
            {focus ? <Minimize2 size={18} /> : <Maximize2 size={18} />}
          </button>
        </div>
        <div className="mode-tabs">
          <button
            className={shown === "pomodoro" ? "selected" : ""}
            disabled={!!active && active.mode !== "pomodoro"}
            onClick={() => {
              setMode("pomodoro");
              setMinutes(data!.settings.work_minutes);
            }}
          >
            <Timer size={16} /> Pomodoro
          </button>
          <button
            className={shown === "stopwatch" ? "selected" : ""}
            disabled={!!active && active.mode !== "stopwatch"}
            onClick={() => {
              setMode("stopwatch");
              setKind("work");
            }}
          >
            <Clock3 size={16} /> Kronometre
          </button>
        </div>
        <div className="timer-dial">
          <svg viewBox="0 0 300 300" aria-hidden="true">
            <defs>
              <linearGradient
                id={compact ? "ring-small" : "ring"}
                x1="0"
                y1="1"
                x2="1"
                y2="0"
              >
                <stop stopColor="#7773f5" />
                <stop offset="1" stopColor="#be9bff" />
              </linearGradient>
            </defs>
            <circle className="dial-track" cx="150" cy="150" r="137" />
            <circle
              className="dial-progress"
              cx="150"
              cy="150"
              r="137"
              stroke={`url(#${compact ? "ring-small" : "ring"})`}
              strokeDasharray={861}
              strokeDashoffset={shown === "stopwatch" ? 0 : 861 * progress}
            />
          </svg>
          <div className="dial-content">
            <span className="dial-label">
              {isBreak
                ? "NEFES AL"
                : shown === "pomodoro"
                  ? "ODAK SÜRESİ"
                  : "AKTİF OTURUM"}
            </span>
            <strong>
              {clock(
                shown === "pomodoro"
                  ? Math.ceil(remaining / 1000) * 1000
                  : remaining,
              )}
            </strong>
            <span className="dial-caption">
              {active?.status === "paused"
                ? "Duraklatıldı · Hazır olduğunda devam et"
                : active?.status === "running"
                  ? "Şu an yaptığın şey önemli."
                  : "Bir sonraki adımın burada başlıyor."}
            </span>
          </div>
        </div>
        <div className="timer-settings">
          {!active && shown === "pomodoro" && (
            <div className="presets">
              {[25, 40, 50].map((n) => (
                <button
                  key={n}
                  className={minutes === n && kind === "work" ? "chosen" : ""}
                  onClick={() => {
                    setMinutes(n);
                    setKind("work");
                  }}
                >
                  {n} dk
                </button>
              ))}
              <label className="custom-minutes">
                <input
                  aria-label="Özel dakika"
                  type="number"
                  min="1"
                  max="1440"
                  value={minutes}
                  onChange={(e) => setMinutes(Number(e.target.value))}
                />{" "}
                dk
              </label>
            </div>
          )}
          <label className="category-select">
            <BookOpen size={17} />
            <select
              aria-label="Çalışılacak ders"
              disabled={!!active}
              value={active?.category_id || category}
              onChange={(e) => setCategory(Number(e.target.value))}
            >
              {liveCats.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
            <ChevronDown
              className="category-chevron"
              size={16}
              aria-hidden="true"
            />
          </label>
        </div>
        <div className="timer-controls">
          <button className="primary" disabled={busy || !data} onClick={toggle}>
            {active?.status === "running" ? (
              <Pause size={18} />
            ) : (
              <Play size={18} fill="currentColor" />
            )}
            {active?.status === "running"
              ? "Duraklat"
              : active
                ? "Devam et"
                : isBreak
                  ? "Molayı başlat"
                  : "Odaklanmaya başla"}
          </button>
          {active && (
            <>
              <button className="secondary" disabled={busy} onClick={finish}>
                <Square size={16} /> Bitir
              </button>
              <button
                className="icon-button danger"
                aria-label="Oturumu iptal et"
                title="Oturumu ve süresini iptal et"
                disabled={busy}
                onClick={() => void cancel()}
              >
                <Trash2 size={17} />
              </button>
            </>
          )}
        </div>
        {!active && shown === "pomodoro" && (
          <div className="break-options">
            <button
              onClick={() => changeKind("work")}
              className={!isBreak ? "active-text" : ""}
            >
              Çalışma
            </button>
            <span>·</span>
            <button onClick={() => changeKind("short")}>
              Kısa mola · {data?.settings.short_minutes} dk
            </button>
            <span>·</span>
            <button onClick={() => changeKind("long")}>
              Uzun mola · {data?.settings.long_minutes} dk
            </button>
          </div>
        )}
        <div className="timer-footer">
          <span>
            <span className="tiny-dot" />{" "}
            {active ? "Süre otomatik kaydediliyor" : "Her odak anı birikir"}
          </span>
          <span>
            {shown === "stopwatch"
              ? `Bugün toplam: ${duration(todayMs)}`
              : "Boşluk: başlat / duraklat"}
          </span>
        </div>
        {active && (
          <p className="small muted">
            Mod değiştirmek için mevcut oturumu bitir veya iptal et. Molalar
            çalışma toplamına eklenmez.
          </p>
        )}
      </Card>
    );
  };
  const miniStats = (
    <div className="stats-row">
      <Stat
        icon={<Clock3 />}
        label="Bugünkü çalışma"
        value={duration(todayMs)}
        foot={`${pomos} tamamlanan Pomodoro`}
        accent="purple"
      />
      <Stat
        icon={<Target />}
        label="Günlük hedef"
        value={`%${percent}`}
        foot={`${duration(todayMs)} / ${duration(goal)}`}
        progress={percent}
        accent="blue"
      />
      <Stat
        icon={<Flame />}
        label="Çalışma serisi"
        value={`${streak} gün`}
        foot={streak ? "Ritmini korumaya devam et" : "Bugün ilk adımını at"}
        accent="orange"
      />
    </div>
  );
  if (!data)
    return (
      <div className="loading">
        <div className="brand-icon">
          <Activity />
        </div>
        <h1>FocusFlow</h1>
        <p>{error || "Yerel çalışma kayıtları açılıyor…"}</p>
        {error && <code>cd ~/coding/focusflow && npm run desktop</code>}
      </div>
    );
  return (
    <div
      className={
        "app " +
        (isAndroid() ? "android-app " : "") +
        (focus ? "focus-mode" : "")
      }
    >
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setPage("dashboard");
          }}
        >
          <span className="brand-icon">
            <Activity size={23} />
          </span>
          Focus<span>Flow</span>
        </a>
        <div className="workspace-label">KİŞİSEL ÇALIŞMA ALANIN</div>
        <nav>
          {(
            [
              ["dashboard", LayoutDashboard, "Genel bakış"],
              ["timer", Timer, "Odak zamanı"],
              ["statistics", BarChart3, "İstatistikler"],
              ["history", History, "Çalışma geçmişi"],
              ["categories", BookOpen, "Derslerim"],
            ] as const
          ).map(([id, Icon, label]) => (
            <button
              key={id}
              aria-label={label}
              aria-current={page === id ? "page" : undefined}
              className={page === id ? "nav-active" : ""}
              onClick={() => setPage(id)}
            >
              <Icon size={19} />
              {isAndroid()
                ? {
                    dashboard: "Ana sayfa",
                    timer: "Odak",
                    statistics: "İstatistik",
                    history: "Geçmiş",
                    categories: "Dersler",
                  }[id]
                : label}
              {id === "timer" && active && <span className="nav-dot" />}
            </button>
          ))}
        </nav>
        <div className="sidebar-note">
          <div className="note-icon">✦</div>
          <p>Her gün biraz daha ileri.</p>
          <span>Odağın, en iyi yatırımın.</span>
        </div>
        <div className="sidebar-bottom">
          <button
            aria-label="Ayarlar"
            aria-current={page === "settings" ? "page" : undefined}
            className={page === "settings" ? "nav-active" : ""}
            onClick={() => setPage("settings")}
          >
            <Settings2 size={18} /> Ayarlar
          </button>
          <div className="local-badge">
            <span className="tiny-dot" />
            Yerel ve çevrimdışı <span>v1.1</span>
          </div>
        </div>
      </aside>
      <main>
        <header>
          <div className="breadcrumb">
            Çalışma alanım <ChevronRight size={14} />
            <span>{titles[page][0]}</span>
          </div>
          <div className="header-date">
            <span className="tiny-dot" />{" "}
            {new Date().toLocaleDateString("tr-TR", {
              day: "numeric",
              month: "long",
              year: "numeric",
            })}
            <button
              className="icon-button"
              title={isAndroid() ? "Arka plana al" : "Sistem tepsisine küçült"}
              aria-label={
                isAndroid() ? "Arka plana al" : "Sistem tepsisine küçült"
              }
              onClick={() => void run(() => invoke("hide_window"))}
            >
              <PanelLeftClose size={17} />
            </button>
          </div>
        </header>
        <div className="page-content">
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                {page === "dashboard" ? "BUGÜN SENİN GÜNÜN" : "FOCUSFLOW"}
              </div>
              <h1>
                {titles[page][0]}
                {page === "dashboard" && <span className="sparkle">✦</span>}
              </h1>
              <p>{titles[page][1]}</p>
            </div>
            {page === "dashboard" ? (
              <div className="date-pill">
                <span className="tiny-dot" /> Bugün
              </div>
            ) : page === "timer" ? (
              <button className="secondary" onClick={() => setFocus(!focus)}>
                <Focus size={16} />
                {focus ? "Odak modundan çık" : "Odak modu"}
              </button>
            ) : null}
          </div>
          {(data.recovery || data.notice) && (
            <div className="notice">
              <RotateCcw size={18} />
              <span>
                {data.notice ||
                  "Önceki oturum son kayıtlı noktada duraklatıldı. Uygulamanın kapalı olduğu süre eklenmedi; hazır olduğunda devam et."}
              </span>
            </div>
          )}
          {page === "dashboard" && (
            <>
              {miniStats}
              <div className="dashboard-grid">
                {timerPanel(true)}
                <div className="right-column">
                  <Card>
                    <div className="card-heading">
                      <h2>
                        <BarChart3 size={17} /> Haftalık ritmin
                      </h2>
                      <button
                        className="icon-button"
                        aria-label="İstatistiklere git"
                        onClick={() => setPage("statistics")}
                      >
                        <ArrowUpRight size={18} />
                      </button>
                    </div>
                    <div className="week-total">
                      {duration(weekMs)} <span>bu hafta</span>
                    </div>
                    <Chart data={data} count={7} />
                    <div className="chart-caption">
                      <span className="legend-dot" /> Çalışma süresi{" "}
                      <span>Son 7 gün</span>
                    </div>
                  </Card>
                  <Card className="subjects-card">
                    <div className="card-heading">
                      <h2>
                        <BookOpen size={17} /> Bugün derslerin
                      </h2>
                      <span className="tag">
                        {
                          new Set(
                            days
                              .filter((d) => d.day === today)
                              .map(
                                (d) =>
                                  sessionMap.get(d.session_id)?.category_id,
                              ),
                          ).size
                        }{" "}
                        ders
                      </span>
                    </div>
                    <SubjectBars data={data} from={today} />
                    <button
                      className="text-button full"
                      onClick={() => setPage("categories")}
                    >
                      Tüm dersleri gör <ArrowRight size={14} />
                    </button>
                  </Card>
                </div>
              </div>
              <Card className="recent-card">
                <div className="card-heading">
                  <h2>Son çalışmaların</h2>
                  <button
                    className="text-button"
                    onClick={() => setPage("history")}
                  >
                    Geçmişi gör <ArrowRight size={14} />
                  </button>
                </div>
                <SessionTable
                  sessions={sessions
                    .filter((s) => s.kind === "work")
                    .slice(0, 4)}
                  categories={data.categories}
                />
              </Card>
            </>
          )}
          {page === "timer" && (
            <div className="timer-page">
              {timerPanel()}
              <div className="timer-side">
                <Card>
                  <span className="eyebrow">BUGÜNKÜ EMEĞİN</span>
                  <div className="big-total">{duration(todayMs)}</div>
                  <p className="muted">Pomodoro ve kronometre, tek toplam.</p>
                  <div className="goal-track">
                    <i style={{ width: percent + "%" }} />
                  </div>
                  <div className="spread small">
                    <span>Günlük hedef</span>
                    <span>%{percent}</span>
                  </div>
                </Card>
                <Card>
                  <h2>
                    <Coffee size={18} /> Mola da sürecin bir parçası
                  </h2>
                  <p className="muted">
                    Her {data.settings.long_every} tamamlanan Pomodoro sonrası
                    uzun bir mola ver. Molalar çalışma sürene eklenmez.
                  </p>
                  <div className="pomo-dots">
                    {Array.from(
                      { length: data.settings.long_every },
                      (_, i) => (
                        <span
                          key={i}
                          className={
                            i < pomos % data.settings.long_every ? "filled" : ""
                          }
                        />
                      ),
                    )}
                  </div>
                  <p className="small muted">
                    Bugün {pomos} Pomodoro tamamladın. Molalar elle başlatılır.
                  </p>
                </Card>
                <div className="keyboard-hint">
                  <kbd>Space</kbd> Başlat / duraklat <kbd>F</kbd> Odak modu{" "}
                  <kbd>Esc</kbd> Çık
                </div>
              </div>
            </div>
          )}
          {page === "statistics" && (
            <>
              <div className="stats-row">
                <Stat
                  icon={<Clock3 />}
                  label="Bugün"
                  value={duration(todayMs)}
                  foot={`${pomos} tamamlanan Pomodoro`}
                />
                <Stat
                  icon={<BarChart3 />}
                  label="Bu hafta"
                  value={duration(weekMs)}
                  foot="Pazartesiden bugüne"
                />
                <Stat
                  icon={<Target />}
                  label="Bu ay"
                  value={duration(monthMs)}
                  foot="Takvim ayı toplamı"
                />
              </div>
              <Statistics data={data} />
              <div className="stats-row">
                <Stat
                  icon={<Clock3 />}
                  label="Kronometre toplamı"
                  value={duration(totalMode("stopwatch"))}
                  foot="Tüm zamanlar"
                />
                <Stat
                  icon={<Timer />}
                  label="En uzun çalışma"
                  value={duration(longest)}
                  foot="Aktif oturum dahil, molalar hariç"
                />
                <Stat
                  icon={<Flame />}
                  label="Çalışma serisi"
                  value={`${streak} gün`}
                  foot={`Bugünkü hedefin %${percent} tamamlandı`}
                />
              </div>
            </>
          )}
          {page === "history" && (
            <HistoryPage
              data={data}
              onEdit={setEdit}
              onDelete={async (s) => {
                if (
                  await ask(
                    "Bu çalışma kaydı ve süresi kalıcı olarak silinsin mi?",
                  )
                )
                  void run(
                    () => invoke("delete_session", { id: s.id }),
                    "Kayıt silindi.",
                  );
              }}
              onExport={exportData}
            />
          )}
          {page === "categories" && (
            <Categories data={data} busy={busy} run={run} ask={ask} />
          )}
          {page === "settings" && (
            <SettingsPage
              key={JSON.stringify(data.settings)}
              settings={data.settings}
              busy={busy}
              onSave={(s) =>
                void run(async () => {
                  await invoke("save_settings", { settings: s });
                  if (!active) setMinutes(s.work_minutes);
                }, "Ayarlar kaydedildi.")
              }
              onExport={exportData}
              onRestore={restore}
              onQuit={() => void run(() => invoke("quit"))}
            />
          )}
          <footer>
            <span>
              <span className="tiny-dot" /> Verilerin bu cihazda, kontrol sende.
            </span>
            <span>Bir seferde tek bir adım.</span>
          </footer>
        </div>
      </main>
      {focus && (
        <button
          className="focus-exit secondary"
          onClick={() => setFocus(false)}
        >
          <Minimize2 size={16} /> Odak modundan çık
        </button>
      )}
      {toast && (
        <div className="toast" role="status">
          <CheckCircle2 size={19} />
          {toast}
        </div>
      )}
      {error && (
        <div className="modal-overlay error-overlay">
          <div className="modal" role="alertdialog" aria-modal="true">
            <h2>İşlem tamamlanamadı</h2>
            <p>{error}</p>
            <button className="primary" onClick={() => setError("")}>
              Tamam
            </button>
          </div>
        </div>
      )}
      {confirm && (
        <div className="modal-overlay">
          <div className="modal" role="alertdialog" aria-modal="true">
            <h2>Devam edilsin mi?</h2>
            <p>{confirm.text}</p>
            <div className="form-actions">
              <button
                className="secondary"
                onClick={() => {
                  confirm.resolve(false);
                  setConfirm(undefined);
                }}
              >
                Vazgeç
              </button>
              <button
                className="primary"
                onClick={() => {
                  confirm.resolve(true);
                  setConfirm(undefined);
                }}
              >
                Onayla
              </button>
            </div>
          </div>
        </div>
      )}
      {edit && (
        <EditModal
          session={edit}
          categories={data.categories}
          onClose={() => setEdit(undefined)}
          onSave={async (payload) => {
            try {
              await invoke("edit_session", payload);
              await refresh();
              setEdit(undefined);
              setToast("Çalışma kaydı güncellendi.");
            } catch (e) {
              setError(String(e));
            }
          }}
        />
      )}
    </div>
  );
}
function Stat({
  icon,
  label,
  value,
  foot,
  progress,
  accent = "purple",
}: {
  icon: ReactNode;
  label: string;
  value: string;
  foot: string;
  progress?: number;
  accent?: string;
}) {
  return (
    <Card className="stat">
      <div className={"stat-icon " + accent}>{icon}</div>
      <div className="stat-body">
        <span>{label}</span>
        <strong>{value}</strong>
        {progress !== undefined ? (
          <div className="goal-track">
            <i style={{ width: progress + "%" }} />
          </div>
        ) : null}
        <small>{foot}</small>
      </div>
    </Card>
  );
}
function Chart({
  data,
  count = 7,
  weekly = false,
}: {
  data: Snapshot;
  count?: number;
  weekly?: boolean;
}) {
  const entries = Array.from({ length: count }, (_, i) => {
    const d = new Date();
    d.setDate(d.getDate() - (count - 1 - i) * (weekly ? 7 : 1));
    if (weekly) d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
    const key = dayKey(d);
    const end = new Date(d);
    end.setDate(end.getDate() + 6);
    const endKey = dayKey(end);
    const ms = data.days
      .filter((p) => (weekly ? p.day >= key && p.day <= endKey : p.day === key))
      .reduce((a, p) => a + p.duration_ms, 0);
    return {
      key,
      ms,
      label: d.toLocaleDateString(
        "tr-TR",
        weekly ? { day: "numeric", month: "short" } : { weekday: "short" },
      ),
      today: key === dayKey(),
    };
  });
  const max = Math.max(
    3600000,
    Math.ceil(Math.max(0, ...entries.map((e) => e.ms)) / 3600000) * 3600000,
  );
  return (
    <div
      className={"chart " + (count > 10 ? "dense" : "")}
      role="img"
      aria-label={entries.map((e) => `${e.key}: ${duration(e.ms)}`).join("; ")}
    >
      <div className="chart-axis">
        <span>{Math.ceil(max / 3600000)} sa</span>
        <span>{(max / 7200000).toLocaleString("tr-TR")} sa</span>
        <span>0</span>
      </div>
      <div className="plot">
        <div className="grid-lines">
          <i />
          <i />
          <i />
        </div>
        {entries.map((e, i) => (
          <div
            key={e.key}
            className={"bar-column " + (e.today ? "today" : "")}
            title={`${e.key}: ${duration(e.ms)}`}
          >
            <div className="bar-slot">
              <div
                className="bar"
                style={{
                  height: (e.ms / max) * 100 + "%",
                  minHeight: e.ms ? "3px" : "0",
                }}
              >
                <span>{duration(e.ms)}</span>
              </div>
            </div>
            <label>
              {count > 10
                ? i % 5 === 0
                  ? new Date(e.key + "T12:00:00").getDate()
                  : ""
                : e.label}
            </label>
          </div>
        ))}
      </div>
    </div>
  );
}
function SubjectBars({
  data,
  from,
  to = dayKey(),
}: {
  data: Snapshot;
  from: string;
  to?: string;
}) {
  const totals = new Map<number, number>();
  const sessions = new Map(data.sessions.map((s) => [s.id, s]));
  data.days
    .filter((d) => d.day >= from && d.day <= to)
    .forEach((d) => {
      const id = sessions.get(d.session_id)?.category_id;
      if (id) totals.set(id, (totals.get(id) || 0) + d.duration_ms);
    });
  const sorted = [...totals].sort((a, b) => b[1] - a[1]);
  const total = sorted.reduce((a, s) => a + s[1], 0);
  return (
    <div className="subject-bars">
      {sorted.length ? (
        sorted.map(([id, ms]) => {
          const c = data.categories.find((c) => c.id === id);
          return (
            <div className="subject" key={id}>
              <div className="spread">
                <span>
                  <i style={{ background: c?.color }} />
                  {c?.name}
                </span>
                <strong>{duration(ms)}</strong>
              </div>
              <div className="subject-track">
                <i
                  style={{
                    width: (ms / total) * 100 + "%",
                    background: c?.color,
                  }}
                />
              </div>
            </div>
          );
        })
      ) : (
        <div className="empty">
          <BookOpen size={25} />
          <p>Bugün yeni bir sayfa.</p>
          <span>İlk çalışmanı başlattığında derslerin burada görünecek.</span>
        </div>
      )}
    </div>
  );
}
function SessionTable({
  sessions,
  categories,
  onEdit,
  onDelete,
}: {
  sessions: Session[];
  categories: Category[];
  onEdit?: (s: Session) => void;
  onDelete?: (s: Session) => void;
}) {
  return (
    <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th>Ders / kategori</th>
            <th>Tarih ve saat</th>
            <th>Mod</th>
            <th>Süre</th>
            <th>Durum</th>
            {onEdit && <th aria-label="İşlemler" />}
          </tr>
        </thead>
        <tbody>
          {sessions.map((s) => {
            const c = categories.find((c) => c.id === s.category_id);
            const active = ["running", "paused"].includes(s.status);
            return (
              <tr key={s.id}>
                <td>
                  <span className="table-subject">
                    <i style={{ background: c?.color }} />
                    {c?.name}
                    {s.kind === "break" && <small> · Mola</small>}
                  </span>
                </td>
                <td>
                  <span>{dateTime(s.started_at)}</span>
                  <small className="table-end">
                    {s.ended_at
                      ? `→ ${dateTime(s.ended_at)}`
                      : "→ devam ediyor"}
                  </small>
                </td>
                <td>
                  <span className={"mode-chip " + s.mode}>
                    {s.mode === "pomodoro" ? (
                      <Timer size={13} />
                    ) : (
                      <Clock3 size={13} />
                    )}{" "}
                    {modeName(s.mode)}
                  </span>
                </td>
                <td className="mono">{clock(s.elapsed_ms)}</td>
                <td>
                  <span className={"status " + s.status}>
                    <span />
                    {statusName(s.status)}
                  </span>
                </td>
                {onEdit && (
                  <td>
                    <div className="row-actions">
                      <button
                        aria-label="Kaydı düzenle"
                        title="Kaydı düzenle"
                        className="icon-button"
                        disabled={active}
                        onClick={() => onEdit(s)}
                      >
                        <Pencil size={14} />
                      </button>
                      <button
                        aria-label="Kaydı sil"
                        title="Kaydı sil"
                        className="icon-button danger"
                        disabled={active}
                        onClick={() => onDelete?.(s)}
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </td>
                )}
              </tr>
            );
          })}
        </tbody>
      </table>
      {!sessions.length && (
        <div className="empty">
          <History size={27} />
          <p>Henüz bir çalışma kaydı yok.</p>
          <span>İlk oturumunla başlayabilirsin.</span>
        </div>
      )}
    </div>
  );
}
function Statistics({ data }: { data: Snapshot }) {
  const [range, setRange] = useState("7");
  const [period, setPeriod] = useState("today");
  const start = new Date();
  if (period === "week")
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  if (period === "month") start.setDate(1);
  const from = dayKey(start);
  const total = (m: string) =>
    data.sessions
      .filter((s) => s.kind === "work" && s.mode === m)
      .reduce((a, s) => a + s.elapsed_ms, 0);
  const p = total("pomodoro"),
    s = total("stopwatch"),
    sum = p + s;
  return (
    <>
      <Card>
        <div className="card-heading">
          <div>
            <h2>Çalışma ritmin</h2>
            <p className="muted small">Aktif oturum dahil, molalar hariç.</p>
          </div>
          <div className="segmented">
            {[
              ["7", "7 gün"],
              ["30", "30 gün"],
              ["week", "Haftalık"],
            ].map(([v, l]) => (
              <button
                key={v}
                className={range === v ? "selected" : ""}
                onClick={() => setRange(v)}
              >
                {l}
              </button>
            ))}
          </div>
        </div>
        <Chart
          data={data}
          count={range === "week" ? 8 : Number(range)}
          weekly={range === "week"}
        />
      </Card>
      <div className="two-columns">
        <Card>
          <div className="card-heading">
            <h2>Derslere ayrılan süre</h2>
            <select
              aria-label="Ders dağılımı dönemi"
              value={period}
              onChange={(e) => setPeriod(e.target.value)}
            >
              <option value="today">Bugün</option>
              <option value="week">Bu hafta</option>
              <option value="month">Bu ay</option>
            </select>
          </div>
          <SubjectBars data={data} from={from} />
        </Card>
        <Card>
          <h2>
            Çalışma modların <small className="muted">· Tüm zamanlar</small>
          </h2>
          <div className="comparison-total">
            {duration(sum)}
            <span> toplam odak</span>
          </div>
          <div className="comparison-track">
            <i style={{ width: sum ? (p / sum) * 100 + "%" : "0" }} />
            <i style={{ width: sum ? (s / sum) * 100 + "%" : "0" }} />
          </div>
          <div className="comparison-labels">
            <div>
              <span className="legend-dot" /> Pomodoro
              <strong>{duration(p)}</strong>
              <small>%{sum ? Math.round((p / sum) * 100) : 0}</small>
            </div>
            <div>
              <span className="legend-dot blue" /> Kronometre
              <strong>{duration(s)}</strong>
              <small>%{sum ? Math.round((s / sum) * 100) : 0}</small>
            </div>
          </div>
        </Card>
      </div>
    </>
  );
}
function HistoryPage({
  data,
  onEdit,
  onDelete,
  onExport,
}: {
  data: Snapshot;
  onEdit: (s: Session) => void;
  onDelete: (s: Session) => void;
  onExport: (f: string) => void;
}) {
  const [mode, setMode] = useState("all"),
    [cat, setCat] = useState("all"),
    [status, setStatus] = useState("all"),
    [from, setFrom] = useState(""),
    [to, setTo] = useState(""),
    [page, setPage] = useState(0);
  const filtered = data.sessions.filter(
    (s) =>
      (mode === "all" || s.mode === mode) &&
      (cat === "all" || s.category_id === Number(cat)) &&
      (status === "all" || s.status === status) &&
      (!from || dayKey(new Date(s.started_at)) >= from) &&
      (!to || dayKey(new Date(s.started_at)) <= to),
  );
  const current = Math.min(
    page,
    Math.max(0, Math.ceil(filtered.length / 15) - 1),
  );
  return (
    <Card>
      <div className="card-heading">
        <h2>
          Tüm oturumlar <span className="tag">{filtered.length}</span>
        </h2>
        <button className="secondary" onClick={() => onExport("csv")}>
          <Download size={15} /> CSV dışa aktar
        </button>
      </div>
      <div className="filters">
        <select
          aria-label="Mod filtresi"
          value={mode}
          onChange={(e) => {
            setMode(e.target.value);
            setPage(0);
          }}
        >
          <option value="all">Tüm modlar</option>
          <option value="pomodoro">Pomodoro</option>
          <option value="stopwatch">Kronometre</option>
        </select>
        <select
          aria-label="Ders filtresi"
          value={cat}
          onChange={(e) => {
            setCat(e.target.value);
            setPage(0);
          }}
        >
          <option value="all">Tüm dersler</option>
          {data.categories.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
              {c.archived ? " (silindi)" : ""}
            </option>
          ))}
        </select>
        <select
          aria-label="Durum filtresi"
          value={status}
          onChange={(e) => {
            setStatus(e.target.value);
            setPage(0);
          }}
        >
          <option value="all">Tüm durumlar</option>
          {["completed", "early", "running", "paused"].map((v) => (
            <option key={v} value={v}>
              {statusName(v)}
            </option>
          ))}
        </select>
        <label>
          Başlangıç
          <input
            aria-label="Başlangıç tarihi"
            type="date"
            value={from}
            onChange={(e) => {
              setFrom(e.target.value);
              setPage(0);
            }}
          />
        </label>
        <label>
          Bitiş
          <input
            aria-label="Bitiş tarihi"
            type="date"
            value={to}
            onChange={(e) => {
              setTo(e.target.value);
              setPage(0);
            }}
          />
        </label>
      </div>
      <SessionTable
        sessions={filtered.slice(current * 15, current * 15 + 15)}
        categories={data.categories}
        onEdit={onEdit}
        onDelete={onDelete}
      />
      <div className="pagination">
        <span>{filtered.length} kayıt · CSV tüm kayıtları içerir.</span>
        <div>
          <button
            className="icon-button"
            aria-label="Önceki sayfa"
            disabled={!current}
            onClick={() => setPage(current - 1)}
          >
            <ChevronLeft size={18} />
          </button>
          <span>
            {current + 1} / {Math.max(1, Math.ceil(filtered.length / 15))}
          </span>
          <button
            className="icon-button"
            aria-label="Sonraki sayfa"
            disabled={(current + 1) * 15 >= filtered.length}
            onClick={() => setPage(current + 1)}
          >
            <ChevronRight size={18} />
          </button>
        </div>
      </div>
    </Card>
  );
}
function Categories({
  data,
  busy,
  run,
  ask,
}: {
  data: Snapshot;
  busy: boolean;
  run: (f: () => Promise<unknown>, m?: string) => Promise<void>;
  ask: (s: string) => Promise<boolean>;
}) {
  const [editing, setEditing] = useState<Category | null>();
  const [name, setName] = useState("");
  const [color, setColor] = useState("#a99bff");
  const today = dayKey();
  const totals = new Map<number, number>();
  data.days
    .filter((d) => d.day === today)
    .forEach((d) => {
      const s = data.sessions.find((s) => s.id === d.session_id);
      if (s)
        totals.set(
          s.category_id,
          (totals.get(s.category_id) || 0) + d.duration_ms,
        );
    });
  return (
    <>
      <div className="section-toolbar">
        <p className="muted">Silinen dersler geçmiş kayıtlarında korunur.</p>
        <button
          className="primary"
          onClick={() => {
            setEditing(null);
            setName("");
            setColor("#a99bff");
          }}
        >
          <Plus size={16} /> Ders ekle
        </button>
      </div>
      <div className="category-grid">
        {data.categories
          .filter((c) => !c.archived)
          .map((c) => (
            <Card key={c.id}>
              <div className="card-heading">
                <div
                  className="course-icon"
                  style={{ color: c.color, background: c.color + "18" }}
                >
                  <BookOpen size={23} />
                </div>
                <div className="row-actions">
                  <button
                    className="icon-button"
                    aria-label={c.name + " düzenle"}
                    onClick={() => {
                      setEditing(c);
                      setName(c.name);
                      setColor(c.color);
                    }}
                  >
                    <Pencil size={16} />
                  </button>
                  <button
                    className="icon-button danger"
                    aria-label={c.name + " sil"}
                    onClick={async () => {
                      if (
                        await ask(
                          `${c.name} ders listesinden kaldırılsın mı? Geçmiş çalışmalar korunacak.`,
                        )
                      )
                        void run(
                          () => invoke("archive_category", { id: c.id }),
                          "Ders silindi.",
                        );
                    }}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
              <h2>{c.name}</h2>
              <strong className="course-total">
                {duration(totals.get(c.id) || 0)}
              </strong>
              <span className="muted small"> bugün</span>
            </Card>
          ))}
      </div>
      {editing !== undefined && (
        <div className="modal-overlay">
          <form
            className="modal"
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () => {
                await invoke("save_category", {
                  id: editing?.id || null,
                  name,
                  color,
                });
                setEditing(undefined);
              }, "Ders kaydedildi.");
            }}
          >
            <div className="card-heading">
              <h2>{editing ? "Dersi düzenle" : "Yeni ders"}</h2>
              <button
                type="button"
                className="icon-button"
                aria-label="Kapat"
                onClick={() => setEditing(undefined)}
              >
                <X size={18} />
              </button>
            </div>
            <label>
              Ders adı
              <input
                autoFocus
                required
                maxLength={80}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Örn. Medical Biology"
              />
            </label>
            <label>
              Renk
              <input
                type="color"
                value={color}
                onChange={(e) => setColor(e.target.value)}
              />
            </label>
            <button className="primary" disabled={busy}>
              <Check size={16} /> Kaydet
            </button>
          </form>
        </div>
      )}
    </>
  );
}
function EditModal({
  session: s,
  categories,
  onClose,
  onSave,
}: {
  session: Session;
  categories: Category[];
  onClose: () => void;
  onSave: (p: Record<string, unknown>) => Promise<void>;
}) {
  const [category, setCategory] = useState(s.category_id),
    [start, setStart] = useState(localInput(s.started_at)),
    [end, setEnd] = useState(localInput(s.ended_at!)),
    [seconds, setSeconds] = useState(s.elapsed_ms / 1000),
    [saving, setSaving] = useState(false);
  return (
    <div className="modal-overlay">
      <form
        className="modal"
        onSubmit={async (e) => {
          e.preventDefault();
          setSaving(true);
          await onSave({
            id: s.id,
            categoryId: category,
            start:
              start === localInput(s.started_at)
                ? s.started_at
                : new Date(start).getTime(),
            end:
              end === localInput(s.ended_at!)
                ? s.ended_at
                : new Date(end).getTime(),
            duration: Math.round(seconds * 1000),
          });
          setSaving(false);
        }}
      >
        <div className="card-heading">
          <h2>Çalışmayı düzenle</h2>
          <button
            type="button"
            className="icon-button"
            aria-label="Kapat"
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </div>
        <label>
          Ders
          <select
            aria-label="Ders"
            value={category}
            onChange={(e) => setCategory(Number(e.target.value))}
          >
            {categories.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Başlangıç
          <input
            required
            type="datetime-local"
            step="1"
            value={start}
            onChange={(e) => setStart(e.target.value)}
          />
        </label>
        <label>
          Bitiş
          <input
            required
            type="datetime-local"
            step="1"
            value={end}
            onChange={(e) => setEnd(e.target.value)}
          />
        </label>
        <label>
          Gerçek çalışma süresi (saniye)
          <input
            required
            type="number"
            min="0.001"
            step="0.001"
            value={seconds}
            onChange={(e) => setSeconds(Number(e.target.value))}
          />
        </label>
        <p className="small muted">
          Tarih veya süre değişirse çalışma süresi bu tarih aralığına orantılı
          dağıtılır. Yalnızca ders değişikliği mevcut duraklamaları korur.
        </p>
        <button className="primary" disabled={saving}>
          Değişiklikleri kaydet
        </button>
      </form>
    </div>
  );
}
function SettingsPage({
  settings,
  busy,
  onSave,
  onExport,
  onRestore,
  onQuit,
}: {
  settings: Settings;
  busy: boolean;
  onSave: (s: Settings) => void;
  onExport: (f: string) => void;
  onRestore: () => void;
  onQuit: () => void;
}) {
  const [s, setS] = useState(settings),
    [auto, setAuto] = useState(false),
    [autoError, setAutoError] = useState(""),
    [path, setPath] = useState("");
  useEffect(() => {
    if (!isAndroid())
      void isEnabled()
        .then(setAuto)
        .catch((e) => setAutoError(String(e)));
    void invoke<string>("data_path")
      .then(setPath)
      .catch((e) => setAutoError(String(e)));
  }, []);
  return (
    <div className="settings-grid">
      <Card>
        <h2>
          <Timer size={19} /> Çalışma ritmi
        </h2>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            onSave(s);
          }}
        >
          {(
            [
              ["work_minutes", "Pomodoro süresi (dakika)", 1440],
              ["short_minutes", "Kısa mola (dakika)", 240],
              ["long_minutes", "Uzun mola (dakika)", 240],
              ["long_every", "Uzun mola aralığı (Pomodoro)", 12],
              ["goal_minutes", "Günlük hedef (dakika)", 1440],
            ] as const
          ).map(([k, label, max]) => (
            <label className="setting-row" key={k}>
              <span>{label}</span>
              <input
                required
                type="number"
                min="1"
                max={max}
                value={s[k]}
                onChange={(e) => setS({ ...s, [k]: Number(e.target.value) })}
              />
            </label>
          ))}
          <p className="small muted">
            Ayarlar sonraki oturuma uygulanır. Molayı kendin başlatırsın.
          </p>
          <button className="primary" disabled={busy}>
            <Check size={16} /> Ayarları kaydet
          </button>
        </form>
      </Card>
      <div className="right-column">
        <SyncPanel />
        <Card>
          <h2>
            <Download size={18} /> Veriler ve yedekleme
          </h2>
          <p className="muted">
            JSON yedeği tüm kayıtları, dersleri ve ayarları içerir. CSV, tablo
            uygulamalarında incelemek içindir.
          </p>
          <div className="stack-buttons">
            <button
              className="secondary"
              disabled={busy}
              onClick={() => onExport("json")}
            >
              <Download size={16} /> JSON yedeği oluştur
            </button>
            <button
              className="secondary"
              disabled={busy}
              onClick={() => onExport("csv")}
            >
              <Download size={16} /> CSV dışa aktar
            </button>
            <button className="secondary" disabled={busy} onClick={onRestore}>
              <Upload size={16} /> Yedekten geri yükle
            </button>
          </div>
          <p className="small muted path">Veri dizini: {path}</p>
        </Card>
        <Card>
          <h2>
            <Settings2 size={18} /> {isAndroid() ? "Telefon" : "Masaüstü"}
          </h2>
          {!isAndroid() && (
            <label className="setting-row">
              <span>Bilgisayar açılışında başlat</span>
              <input
                type="checkbox"
                checked={auto}
                onChange={async (e) => {
                  try {
                    if (e.target.checked) await enable();
                    else await disable();
                    setAuto(await isEnabled());
                    setAutoError("");
                  } catch (err) {
                    setAutoError(String(err));
                  }
                }}
              />
            </label>
          )}
          {autoError && (
            <p role="alert" className="danger">
              {autoError}
            </p>
          )}
          {isAndroid() && (
            <>
              <p className="small muted">
                Pomodoro bitiş uyarıları için bildirimlere izin ver. Telefonun
                arka plan uygulamalarını kısıtlıyorsa FocusFlow için pil
                kullanımını sınırsız seç.
              </p>
              <button
                className="secondary"
                onClick={() =>
                  void invoke("open_app_settings").catch((e) =>
                    setAutoError(String(e)),
                  )
                }
              >
                <Settings2 size={16} /> Android uygulama ayarları
              </button>
            </>
          )}
          <p className="small muted">
            {isAndroid()
              ? "Ekran kapalıyken çalışma devam eder. Aktif oturum bildirim alanında görünür. Android uygulamayı zorla kapatırsa son kaydedilen süre korunur ve yeniden açıldığında oturum duraklatılır. Aşağıdaki düğme oturumu kaydedip duraklatır."
              : "Pencereyi kapatmak sistem tepsisine küçültür; çalışma devam eder. Tam çıkışta oturum kaydedilip duraklatılır."}
          </p>
          <button className="secondary" disabled={busy} onClick={onQuit}>
            <Power size={16} /> Kaydet ve uygulamadan çık
          </button>
        </Card>
      </div>
    </div>
  );
}
export default App;
