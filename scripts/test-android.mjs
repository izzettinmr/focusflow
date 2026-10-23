// Real Android WebView -> Java bridge -> JNI -> the shared Rust/SQLite engine.
// Uses only the named emulator and a separate audit package.
import { _android } from "playwright";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, writeFile } from "node:fs/promises";
import assert from "node:assert/strict";
if (process.argv.includes("--files")) {
  await import("./test-android-files.mjs");
  process.exit(0);
}
const exec = promisify(execFile),
  serial = "emulator-5556",
  pkg = "com.focusflow.mobile.audit";
const adb = (...args) =>
  exec("adb", ["-s", serial, ...args], {
    timeout: 90000,
    maxBuffer: 4 * 1024 * 1024,
  });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function capture(name) {
  try {
    const shot = await exec(
      "adb",
      ["-s", serial, "exec-out", "screencap", "-p"],
      { encoding: "buffer", timeout: 15000, maxBuffer: 16 * 1024 * 1024 },
    );
    await writeFile(`android/test-results/${name}.png`, shot.stdout);
  } catch (e) {
    console.log("Optional native screenshot unavailable:", name);
  }
}

await mkdir("android/test-results", { recursive: true });
let device;
const results = {};
try {
  await adb("wait-for-device");
  const name = (await adb("emu", "avd", "name")).stdout;
  assert(
    name.includes("FocusFlowAudit20261022"),
    "Refusing to use a different device",
  );
  for (let i = 0; i < 120; i++) {
    if (
      (await adb("shell", "getprop", "sys.boot_completed")).stdout.trim() ===
      "1"
    )
      break;
    if (i === 119) throw Error("Android boot timeout");
    await sleep(2000);
  }
  await adb("shell", "input", "keyevent", "KEYCODE_WAKEUP");
  await adb("shell", "wm", "dismiss-keyguard");
  // Test fixture needs no Google background services; keep the small emulator responsive.
  for (const app of [
    "com.google.android.gms",
    "com.google.android.apps.wellbeing",
    "com.google.android.googlequicksearchbox",
    "com.google.android.apps.nexuslauncher",
  ]) {
    await adb("shell", "pm", "disable-user", "--user", "0", app).catch(
      () => {},
    );
  }
  console.log("Installing test APK on the ready emulator…");
  await adb(
    "install",
    "--no-streaming",
    "-r",
    "android/output/FocusFlow-audit.apk",
  );
  await adb("shell", "pm", "clear", pkg);
  await adb(
    "shell",
    "pm",
    "grant",
    pkg,
    "android.permission.POST_NOTIFICATIONS",
  );
  console.log("Starting native FocusFlow…");
  await adb(
    "shell",
    "am",
    "start",
    "-W",
    "-n",
    `${pkg}/com.focusflow.mobile.MainActivity`,
  );
  let pid;
  for (let i = 0; i < 30; i++) {
    pid = (
      await adb("shell", "pidof", pkg).catch(() => ({ stdout: "" }))
    ).stdout.trim();
    if (pid) break;
    await sleep(1000);
  }
  assert(pid, "App did not start");
  device = (await _android.devices({ omitDriverInstall: true })).find(
    (d) => d.serial() === serial,
  );
  assert(device, "Named emulator not found by Playwright");
  const page = await (await device.webView({ pkg }, { timeout: 60000 })).page();
  console.log("Connected to the real Android WebView.");
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page
    .getByRole("heading", { name: "Genel bakış" })
    .waitFor({ timeout: 90000 });
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
    false,
  );
  await capture("dashboard");
  await page.evaluate(() => {
    const original = window.__focusflowResult,
      pending = new Map();
    let next = 900000;
    window.__focusflowResult = (id, r) => {
      if (pending.has(id)) {
        const p = pending.get(id);
        pending.delete(id);
        r.ok ? p.resolve(r.value) : p.reject(Error(r.error));
      } else original(id, r);
    };
    window.nativeTest = (cmd, args = {}) =>
      new Promise((resolve, reject) => {
        const id = String(++next);
        pending.set(id, { resolve, reject });
        window.FocusFlowAndroid.request(id, cmd, JSON.stringify(args));
      });
  });
  const native = (cmd, args = {}) =>
    page.evaluate(({ cmd, args }) => window.nativeTest(cmd, args), {
      cmd,
      args,
    });
  const stopwatchStartWall = Date.now();
  await native("start", {
    mode: "stopwatch",
    kind: "work",
    categoryId: 5,
    minutes: 0,
  });
  await sleep(2500);
  await adb("shell", "input", "keyevent", "KEYCODE_SLEEP");
  console.log("Real stopwatch running with screen off for 20 seconds…");
  await sleep(20000);
  await adb("shell", "input", "keyevent", "KEYCODE_WAKEUP");
  await adb("shell", "wm", "dismiss-keyguard");
  await native("action", { action: "pause" });
  const a = await native("snapshot"),
    paused = a.sessions[0].elapsed_ms;
  assert(
    paused >= 21000 && paused <= Date.now() - stopwatchStartWall + 3000,
    `Screen-off elapsed: ${paused}`,
  );
  await sleep(2500);
  assert.equal((await native("snapshot")).sessions[0].elapsed_ms, paused);
  await native("action", { action: "resume" });
  await sleep(1500);
  await native("action", { action: "finish" });
  results.stopwatch_screen_off_pause_resume = true;
  await native("start", {
    mode: "pomodoro",
    kind: "work",
    categoryId: 5,
    minutes: 1,
  });
  await assert.rejects(() =>
    native("start", {
      mode: "stopwatch",
      kind: "work",
      categoryId: 1,
      minutes: 0,
    }),
  );
  const services = (await adb("shell", "dumpsys", "activity", "services", pkg))
    .stdout;
  assert(
    services.includes("TimerService") && services.includes("isForeground=true"),
    "Foreground service missing",
  );
  await adb("shell", "input", "keyevent", "KEYCODE_SLEEP");
  console.log("One-minute Pomodoro finishing with screen off…");
  await sleep(62000);
  const notifications = (
    await adb("shell", "dumpsys", "notification", "--noredact")
  ).stdout;
  assert(
    notifications.includes("Pomodoro tamamlandı"),
    "Native completion notification missing",
  );
  await adb("shell", "input", "keyevent", "KEYCODE_WAKEUP");
  await adb("shell", "wm", "dismiss-keyguard");
  const b = await native("snapshot"),
    pomo = b.sessions.find((s) => s.mode === "pomodoro");
  assert.equal(pomo.status, "completed");
  assert.equal(pomo.elapsed_ms, 60000);
  assert.equal(
    b.days.reduce((n, d) => n + d.duration_ms, 0),
    b.sessions.reduce((n, s) => n + s.elapsed_ms, 0),
  );
  results.pomodoro_alarm_notification_single_shared_total = true;
  await native("start", {
    mode: "pomodoro",
    kind: "break",
    categoryId: 1,
    minutes: 1,
  });
  await sleep(1200);
  await native("action", { action: "finish" });
  const c = await native("snapshot");
  assert.equal(
    c.days.reduce((n, d) => n + d.duration_ms, 0),
    b.days.reduce((n, d) => n + d.duration_ms, 0),
  );
  results.break_excluded = true;
  for (const name of [
    "Genel bakış",
    "Odak zamanı",
    "İstatistikler",
    "Çalışma geçmişi",
    "Derslerim",
    "Ayarlar",
  ]) {
    await page.getByRole("button", { name, exact: true }).click();
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
      `${name} overflow`,
    );
  }
  await page
    .getByRole("button", { name: "Android uygulama ayarları" })
    .waitFor();
  assert.equal(await page.getByText("Bilgisayar açılışında başlat").count(), 0);
  await capture("settings");
  await assert.rejects(() => native("export_data", { format: "json" }));
  results.internal_commands_not_exposed_to_js = true;
  const savedSettings = (await native("snapshot")).settings;
  await native("save_settings", {
    settings: { ...savedSettings, language: "en" },
  });
  await page.getByRole("heading", { name: "Settings", exact: true }).waitFor();
  assert.equal(await page.locator("html").getAttribute("lang"), "en");
  await capture("settings-english-native");
  assert.equal((await native("snapshot")).settings.language, "en");
  results.english_ui_and_sqlite = true;

  assert.deepEqual(errors, []);
  await native("start", {
    mode: "stopwatch",
    kind: "work",
    categoryId: 1,
    minutes: 0,
  });
  await sleep(6500);
  // Test only the audit process; force-stop intentionally simulates Android termination.
  await adb("shell", "am", "force-stop", pkg);
  await sleep(2500);
  await adb(
    "shell",
    "am",
    "start",
    "-W",
    "-n",
    `${pkg}/com.focusflow.mobile.MainActivity`,
  );
  await sleep(2000);
  pid = "";
  for (let i = 0; i < 30 && !pid; i++) {
    pid = (
      await adb("shell", "pidof", pkg).catch(() => ({ stdout: "" }))
    ).stdout.trim();
    if (!pid) await sleep(1000);
  }
  assert(pid, "Android did not restart the app");
  const reopened = await (
    await device.webView(
      { socketName: `webview_devtools_remote_${pid}` },
      { timeout: 60000 },
    )
  ).page();
  await reopened.getByRole("button", { name: "Resume", exact: true }).waitFor();
  assert.equal(await reopened.locator("html").getAttribute("lang"), "en");
  results.english_survives_restart = true;
  results.force_stop_recovers_paused = true;
  await capture("recovered");
  const backup = (
    await adb("exec-out", "run-as", pkg, "cat", "files/focusflow.db")
  ).stdout;
  assert(backup.startsWith("SQLite format 3"));
  results.real_sqlite_file = true;
  await writeFile(
    "android/test-results/native-results.json",
    JSON.stringify(results, null, 2),
  );
  console.log("Native Android tests passed", results);
} catch (e) {
  const logs = await adb("logcat", "-d", "-t", "800").catch(() => ({
    stdout: "",
  }));
  await writeFile("android/test-results/logcat.txt", logs.stdout);
  throw e;
} finally {
  if (device) await device.close();
}
