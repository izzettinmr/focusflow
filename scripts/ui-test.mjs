// Test-only IPC adapter: React -> stdio -> gerçek Rust Engine -> geçici SQLite.
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { createInterface } from "node:readline";
import { readFile, mkdtemp, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import assert from "node:assert/strict";
const root = process.cwd(),
  temp = await mkdtemp(path.join(tmpdir(), "focusflow-ui-"));
const mobile = process.env.FOCUSFLOW_MOBILE_TEST === "1";
const child = spawn(
  path.join(root, "src-tauri/target/debug/examples/ui_bridge"),
  [path.join(temp, "test.db")],
  { stdio: ["pipe", "pipe", "inherit"] },
);
const queue = [];
createInterface({ input: child.stdout }).on("line", (line) =>
  queue.shift()?.(JSON.parse(line)),
);
const rpc = (cmd, args = {}) =>
  new Promise((resolve) => {
    queue.push(resolve);
    child.stdin.write(JSON.stringify({ cmd, args }) + "\n");
  });
const server = createServer(async (req, res) => {
  try {
    if (req.method === "POST") {
      let body = "";
      for await (const c of req) body += c;
      const v = JSON.parse(body);
      res.setHeader("content-type", "application/json");
      res.end(JSON.stringify(await rpc(v.cmd, v.args)));
      return;
    }
    let file =
      req.url === "/" ? "index.html" : decodeURIComponent(req.url.slice(1));
    if (file.includes("..")) throw Error("path");
    const data = await readFile(path.join(root, "dist", file));
    res.setHeader(
      "content-type",
      file.endsWith(".js")
        ? "text/javascript"
        : file.endsWith(".css")
          ? "text/css"
          : "text/html",
    );
    res.end(data);
  } catch (e) {
    res.statusCode = 500;
    res.end(String(e));
  }
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
let browser;
try {
  browser = await chromium.launch({
    headless: true,
    executablePath: process.env.FOCUSFLOW_BROWSER,
  });
  const page = await browser.newPage({
    viewport: { width: 1365, height: 1050 },
  });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.addInitScript((mobile) => {
    if (mobile) {
      window.FocusFlowAndroid = {
        request: async (id, cmd, args) => {
          const r = await fetch("/rpc", {
            method: "POST",
            body: JSON.stringify({ cmd, args: JSON.parse(args) }),
          }).then((r) => r.json());
          window.__focusflowResult(
            id,
            "error" in r
              ? { ok: false, error: r.error }
              : { ok: true, value: r.ok },
          );
        },
      };
      return;
    }
    window.isTauri = true;
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        const r = await fetch("/rpc", {
          method: "POST",
          body: JSON.stringify({ cmd, args }),
        }).then((r) => r.json());
        if ("error" in r) throw new Error(r.error);
        return r.ok;
      },
    };
  }, mobile);
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  await page.getByRole("heading", { name: "Genel bakış" }).waitFor();
  await mkdir("test-results", { recursive: true });
  // Language is stored by the real SQLite engine; reload must preserve it.
  await page.getByRole("button", { name: "Ayarlar", exact: true }).click();
  await page.getByLabel("Dil / Language", { exact: true }).selectOption("en");
  await page
    .getByRole("button", { name: "Ayarları kaydet", exact: true })
    .click();
  await page.getByRole("heading", { name: "Settings", exact: true }).waitFor();
  assert.equal((await rpc("snapshot")).ok.settings.language, "en");
  assert.equal(await page.locator("html").getAttribute("lang"), "en");
  await page.reload();
  await page
    .getByRole("heading", { name: /^Overview/ })
    .waitFor()
    .catch(async (e) => {
      console.error(
        "Language reload diagnostics",
        await page.locator("body").innerText(),
        await rpc("snapshot"),
        errors,
      );
      throw e;
    });
  for (const name of [
    "Focus time",
    "Statistics",
    "Study history",
    "My courses",
    "Settings",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
    await page.getByRole("heading", { name, exact: true }).waitFor();
  }
  await page
    .getByLabel("Pairing code from your computer")
    .fill("FF1|8.8.8.8:443|x|x");
  await page
    .getByRole("button", { name: "Pair and sync", exact: true })
    .click();
  await page
    .getByRole("alert")
    .filter({ hasText: "Only local IPv4 connections are allowed" })
    .waitFor();
  for (const width of [390, 800, 1365]) {
    await page.setViewportSize({ width, height: 1050 });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
      `English overflow at ${width}`,
    );
  }
  await page.screenshot({
    path: "test-results/settings-english.png",
    fullPage: true,
  });
  await page.getByLabel("Dil / Language", { exact: true }).selectOption("tr");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await page.getByRole("heading", { name: "Ayarlar", exact: true }).waitFor();
  assert.equal((await rpc("snapshot")).ok.settings.language, "tr");
  await page.getByRole("button", { name: "Genel bakış", exact: true }).click();
  await page.screenshot({ path: "test-results/dashboard.png", fullPage: true });
  await page.getByRole("button", { name: "Odaklanmaya başla" }).click();
  await page.getByRole("button", { name: "Duraklat", exact: true }).waitFor();
  await new Promise((r) => setTimeout(r, 2200));
  await page.getByRole("button", { name: "Duraklat", exact: true }).click();
  await page.getByRole("button", { name: "Devam et", exact: true }).waitFor();
  const a = (await rpc("snapshot")).ok;
  assert.equal(a.sessions.length, 1);
  assert.equal(a.sessions[0].status, "paused");
  const paused = a.sessions[0].elapsed_ms;
  assert(
    paused >= 1500 && paused <= Date.now() - a.sessions[0].started_at + 100,
    `Invalid recorded duration: ${paused}`,
  );
  await new Promise((r) => setTimeout(r, 1500));
  assert.equal((await rpc("snapshot")).ok.sessions[0].elapsed_ms, paused);
  await page.getByRole("button", { name: "Bitir", exact: true }).click();
  await page.getByRole("button", { name: "Odaklanmaya başla" }).waitFor();
  await page.getByRole("button", { name: "Kronometre", exact: true }).click();
  await page.getByRole("button", { name: "Odaklanmaya başla" }).click();
  await page.getByRole("button", { name: "Duraklat", exact: true }).waitFor();
  await new Promise((r) => setTimeout(r, 1500));
  await page.getByRole("button", { name: "Bitir", exact: true }).click();
  await page.getByRole("button", { name: "Odaklanmaya başla" }).waitFor();
  const b = (await rpc("snapshot")).ok;
  assert.equal(b.sessions.length, 2);
  assert.equal(b.sessions.filter((s) => s.status === "early").length, 2);
  assert.equal(
    b.days.reduce((n, d) => n + d.duration_ms, 0),
    b.sessions.reduce((n, s) => n + s.elapsed_ms, 0),
  );
  await page.getByRole("button", { name: "Ayarlar", exact: true }).click();
  await page
    .getByRole("heading", { name: "Wi-Fi eşitleme", exact: true })
    .waitFor();
  await page
    .getByLabel("Bilgisayardaki bağlantı kodu")
    .fill("FF1|8.8.8.8:443|x|x");
  await page
    .getByRole("button", { name: "Eşleştir ve eşitle", exact: true })
    .click();
  await page
    .getByRole("alert")
    .filter({ hasText: "Yalnızca yerel IPv4" })
    .waitFor();
  assert.equal((await rpc("sync_status")).ok.mode, "off");
  if (mobile) {
    assert.equal(
      await page
        .getByRole("button", { name: "Bu bilgisayarda eşitlemeyi aç" })
        .count(),
      0,
    );
  } else if (process.env.FOCUSFLOW_SKIP_HOST_TEST !== "1") {
    await page
      .getByRole("button", {
        name: "Bu bilgisayarda eşitlemeyi aç",
        exact: true,
      })
      .click();
    await page
      .getByText("Telefon bağlantısına hazır", { exact: true })
      .waitFor()
      .catch(async (e) => {
        const status = (await rpc("sync_status")).ok;
        console.error("Host diagnostic", {
          mode: status.mode,
          error: status.error,
        });
        throw e;
      });
    assert.equal((await rpc("sync_status")).ok.mode, "host");
    if (await page.getByLabel("Telefona aktarılacak kod").count()) {
      await page.getByRole("button", { name: "Kodu seç", exact: true }).click();
      assert(
        await page
          .getByLabel("Telefona aktarılacak kod")
          .evaluate(
            (el) => el.selectionEnd - el.selectionStart === el.value.length,
          ),
      );
    }
    await page
      .getByRole("button", { name: "Wi-Fi eşitlemesini kapat", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Eşleştir ve eşitle", exact: true })
      .waitFor();
    assert.equal((await rpc("sync_status")).ok.mode, "off");
  }
  if (mobile) {
    for (const width of [320, 360, 390, 430]) {
      await page.setViewportSize({ width, height: 800 });
      for (const name of [
        "Genel bakış",
        "Odak zamanı",
        "İstatistikler",
        "Çalışma geçmişi",
        "Derslerim",
        "Ayarlar",
      ]) {
        await page.getByRole("button", { name, exact: true }).click();
        await page
          .getByRole("heading", { name, exact: name !== "Genel bakış" })
          .first()
          .waitFor();
        if (name === "Odak zamanı") {
          const reachable = await page
            .getByRole("button", { name: "Odaklanmaya başla", exact: true })
            .evaluate((el) => {
              const r = el.getBoundingClientRect();
              const hit = document.elementFromPoint(
                r.x + r.width / 2,
                r.y + r.height / 2,
              );
              return hit === el || el.contains(hit);
            });
          if (!reachable) {
            console.log(
              await page
                .getByRole("button", { name: "Odaklanmaya başla", exact: true })
                .evaluate((el) => {
                  const r = el.getBoundingClientRect(),
                    nav = document
                      .querySelector(".sidebar")
                      .getBoundingClientRect();
                  const hit = document.elementFromPoint(
                    r.x + r.width / 2,
                    r.y + r.height / 2,
                  );
                  return {
                    width: innerWidth,
                    height: innerHeight,
                    scrollY,
                    button: r.toJSON(),
                    nav: nav.toJSON(),
                    hit: hit?.outerHTML,
                  };
                }),
            );
            await page.screenshot({
              path: `test-results/mobile-layout-${width}.png`,
              fullPage: true,
            });
          }
          assert(
            reachable,
            `${width}px timer start is covered or outside the screen`,
          );
        }
        if (name === "Ayarlar") {
          await page
            .getByRole("button", { name: "Android uygulama ayarları" })
            .waitFor();
          assert.equal(
            await page.getByText("Bilgisayar açılışında başlat").count(),
            0,
          );
        }
        assert.equal(
          await page.evaluate(
            () => document.documentElement.scrollWidth > innerWidth,
          ),
          false,
          `${width}px ${name} overflow`,
        );
      }
    }
    await page
      .getByRole("button", { name: "Odak zamanı", exact: true })
      .click();
    await page.screenshot({
      path: "test-results/android-timer.png",
      fullPage: true,
    });
    assert.equal(
      await page.getByText("Bilgisayar açılışında başlat").count(),
      0,
    );
  }
  await page.getByRole("button", { name: "Derslerim", exact: true }).click();
  await page.getByRole("button", { name: "Ders ekle", exact: true }).click();
  await page.getByLabel("Ders adı").fill("Biyomedikal Test");
  await page.getByRole("button", { name: "Kaydet", exact: true }).click();
  await page.getByRole("heading", { name: "Biyomedikal Test" }).waitFor();
  assert.equal((await rpc("snapshot")).ok.categories.length, 8);
  await page
    .getByRole("button", { name: "Çalışma geçmişi", exact: true })
    .click();
  await page.getByRole("button", { name: "Kaydı düzenle" }).first().click();
  await page.getByLabel("Ders", { exact: true }).selectOption("5");
  await page.getByRole("button", { name: "Değişiklikleri kaydet" }).click();
  await page
    .getByRole("heading", { name: "Çalışmayı düzenle" })
    .waitFor({ state: "hidden" });
  assert.equal((await rpc("snapshot")).ok.sessions[0].category_id, 5);
  await page
    .getByRole("button", { name: "İstatistikler", exact: true })
    .click();
  await page.getByRole("button", { name: "30 gün", exact: true }).click();
  await page.getByRole("button", { name: "Haftalık", exact: true }).click();
  await page.screenshot({
    path: "test-results/statistics.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 800, height: 850 });
  await page.getByRole("button", { name: "Odak zamanı", exact: true }).click();
  await page.screenshot({
    path: "test-results/timer-small.png",
    fullPage: true,
  });
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
    false,
  );
  assert.deepEqual(errors, []);
  console.log(
    "UI testi geçti: Pomodoro, duraklama, kronometre, ortak toplam, ders ekleme, geçmiş düzenleme, grafikler, 800px görünüm.",
  );
  console.log("Geçici test veritabanı:", temp);
} finally {
  if (browser) await browser.close();
  child.stdin.end();
  server.close();
}
