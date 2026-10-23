import { t, message, locale } from "./i18n";
import { useEffect, useRef, useState } from "react";
import { Wifi, RefreshCw, Link2, Unplug, CheckCircle2 } from "lucide-react";
import { invoke, isAndroid } from "./platform";
type Status = {
  mode: "off" | "host" | "client";
  ready: boolean;
  busy: boolean;
  peer: string | null;
  codes: string[];
  last_success: number | null;
  changed: number;
  error: string | null;
};
export function SyncPanel() {
  const [status, setStatus] = useState<Status>();
  const [code, setCode] = useState("");
  const [address, setAddress] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const selected = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    let live = true;
    const refresh = () => {
      if (document.hidden) return;
      void invoke<Status>("sync_status")
        .then((s) => {
          if (live) setStatus(s);
        })
        .catch((e) => {
          if (live) setError(String(e));
        });
    };
    refresh();
    const timer = setInterval(refresh, 3000);
    document.addEventListener("visibilitychange", refresh);
    return () => {
      live = false;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", refresh);
    };
  }, []);
  async function configure(action: string) {
    setBusy(true);
    setError("");
    try {
      setStatus(
        await invoke<Status>("sync_configure", {
          action,
          code: action === "connect" ? code.trim() : null,
        }),
      );
      if (action === "connect") setCode("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="card sync-panel" aria-label={t("Wi-Fi eşitleme")}>
      <h2>
        <Wifi size={19} />
        {" " + t("Wi-Fi eşitleme") + " "}
      </h2>
      <p className="muted">
        {" " +
          t(
            "Telefon ve bilgisayar aynı ağdayken derslerini ve bitmiş çalışmalarını birleştir. Aktif sayaç ve süre ayarları cihazında kalır.",
          ) +
          " "}
      </p>
      {!status && !error && (
        <p role="status">{"" + t("Eşitleme ayarları okunuyor…") + ""}</p>
      )}
      {status?.mode === "off" && (
        <>
          {!isAndroid() && (
            <div className="sync-host-start">
              <button
                className="primary"
                disabled={busy}
                onClick={() => void configure("host")}
              >
                <Wifi size={16} />
                {" " + t("Bu bilgisayarda eşitlemeyi aç") + " "}
              </button>
              <p className="small muted">
                {" " +
                  t(
                    "Açtıktan sonra bağlantı kodunu telefonuna aktar. Bilgisayarda FocusFlow açık kalmalı; sistem tepsisinde çalışabilir.",
                  ) +
                  " "}
              </p>
            </div>
          )}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void configure("connect");
            }}
          >
            <label htmlFor="wifi-pair-code">
              {"" + t("Bilgisayardaki bağlantı kodu") + ""}
            </label>
            <textarea
              id="wifi-pair-code"
              rows={3}
              required
              maxLength={200}
              value={code}
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              placeholder="FF1|192.168.…"
              onChange={(e) => setCode(e.target.value)}
            />
            <button className="secondary" disabled={busy || !code.trim()}>
              <Link2 size={16} />
              {" " + t("Eşleştir ve eşitle") + " "}
            </button>
          </form>
        </>
      )}
      {status?.mode === "host" && (
        <>
          <p className="sync-state" role="status">
            {status.ready
              ? t("Telefon bağlantısına hazır")
              : t("Yerel bağlantı açılıyor…")}
          </p>
          {status.codes.length > 1 && (
            <label>
              {" " + t("Bağlantı adresi") + " "}
              <select
                value={Math.min(address, status.codes.length - 1)}
                onChange={(e) => setAddress(Number(e.target.value))}
              >
                {status.codes.map((c, i) => (
                  <option key={c} value={i}>
                    {c.split("|")[1]}
                  </option>
                ))}
              </select>
            </label>
          )}
          {status.codes.length > 0 ? (
            <>
              <label htmlFor="wifi-host-code">
                {"" + t("Telefona aktarılacak kod") + ""}
              </label>
              <textarea
                ref={selected}
                id="wifi-host-code"
                readOnly
                rows={3}
                value={status.codes[address] || status.codes[0]}
                spellCheck={false}
                onFocus={(e) => e.currentTarget.select()}
              />
              <button
                className="secondary"
                onClick={() => {
                  selected.current?.focus();
                  selected.current?.select();
                }}
              >
                <Link2 size={16} />
                {" " + t("Kodu seç") + " "}
              </button>
              <p className="small muted">
                {" " +
                  t(
                    "Kodu seçip Ctrl+C ile kopyala; telefonunda Ayarlar → Wi-Fi eşitleme alanına yapıştır. Kod yalnızca kendi cihazlarınla paylaşılmalı.",
                  ) +
                  " "}
              </p>
            </>
          ) : (
            <p role="status">
              {" " +
                t(
                  "Yerel IPv4 adresi bulunamadı. Bilgisayarı Wi-Fi ağına bağla.",
                ) +
                " "}
            </p>
          )}
        </>
      )}
      {status?.mode === "client" && (
        <>
          <p className="sync-state" role="status">
            {status.busy
              ? t("Eşitleniyor…")
              : status.ready
                ? t("Cihazlar eşitlendi")
                : t("Bilgisayar bağlantısı bekleniyor")}
          </p>
          <p className="small muted path">
            {"" + t("Bilgisayar:") + " "}
            {status.peer}
          </p>
          <button
            className="secondary"
            disabled={busy || status.busy}
            onClick={() => void configure("now")}
          >
            <RefreshCw size={16} />
            {" " + t("Şimdi eşitle") + " "}
          </button>
          <p className="small muted">
            {" " +
              t(
                "Bağlıyken yaklaşık 30 saniyede bir kontrol edilir. Uygulama kapatılmışsa sonraki açılışta yeniden denenir.",
              ) +
              " "}
          </p>
        </>
      )}
      {status?.last_success && (
        <p className="sync-success">
          <CheckCircle2 size={15} />
          {" " + t("Son eşitleme:") + ""}{" "}
          {new Date(status.last_success).toLocaleTimeString(locale())} ·{" "}
          {status.changed}
          {" " + t("kayıt değişti") + " "}
        </p>
      )}
      {(error || status?.error) && (
        <p role="alert" className="danger sync-error">
          {message(error || status?.error || "")}
        </p>
      )}
      {status && status.mode !== "off" && (
        <button
          className="secondary"
          disabled={busy}
          onClick={() => void configure("off")}
        >
          <Unplug size={16} />{" "}
          {status.mode === "host"
            ? t("Wi-Fi eşitlemesini kapat")
            : t("Cihaz bağlantısını kaldır")}
        </button>
      )}
      <p className="small muted">
        {" " +
          t(
            "Çakışan çalışma saatleri varsa eşitleme uygulanmaz; Geçmiş ekranından kayıtları düzeltip yeniden dene. Bulut hesabı gerekmez.",
          ) +
          " "}
      </p>
    </section>
  );
}
