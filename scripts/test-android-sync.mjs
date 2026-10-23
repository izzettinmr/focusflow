// Real Android JNI client <-> native Rust host; only the named test emulator.
import { _android } from "playwright";
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { createInterface } from "node:readline";
import { mkdtemp, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import assert from "node:assert/strict";
const exec = promisify(execFile),
  serial = "emulator-5556",
  pkg = "com.focusflow.mobile.audit";
const adb = (...args) =>
  exec("adb", ["-s", serial, ...args], {
    timeout: 90000,
    maxBuffer: 8 * 1024 * 1024,
  });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function until(fn, label) {
  for (let i = 0; i < 40; i++) {
    const result = await fn();
    if (result) return result;
    await sleep(500);
  }
  throw Error(label);
}
assert(
  (await adb("emu", "avd", "name")).stdout.includes("FocusFlowAudit20261022"),
);
const dir = await mkdtemp(path.join(tmpdir(), "focusflow-sync-android-"));
const host = spawn("src-tauri/target/debug/examples/sync_host", [dir], {
  stdio: ["pipe", "pipe", "inherit"],
});
const queue = [];
createInterface({ input: host.stdout }).on("line", (line) => {
  const r = JSON.parse(line),
    p = queue.shift();
  r.error ? p.reject(Error(r.error)) : p.resolve(r.ok);
});
const rpc = (cmd) =>
  new Promise((resolve, reject) => {
    queue.push({ resolve, reject });
    host.stdin.write(JSON.stringify({ cmd }) + "\n");
  });
let device;
const results = {};
try {
  const status = await until(async () => {
    const s = await rpc("status");
    return s.ready && s.codes.length ? s : null;
  }, "Native host did not start");
  // Emulator uses the documented host-loopback alias; the certificate and secret stay identical.
  const code = status.codes[0].replace(/\|[^|]+\|/, "|10.0.2.2:45873|");
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
  await adb("shell", "input", "keyevent", "KEYCODE_WAKEUP");
  await adb("shell", "wm", "dismiss-keyguard");
  await adb(
    "shell",
    "am",
    "start",
    "-n",
    `${pkg}/com.focusflow.mobile.MainActivity`,
  );
  device = (await _android.devices({ omitDriverInstall: true })).find(
    (d) => d.serial() === serial,
  );
  assert(device);
  let page = await (await device.webView({ pkg }, { timeout: 60000 })).page();
  async function attach() {
    await page
      .getByRole("heading", { name: "Genel bakış" })
      .waitFor({ timeout: 60000 });
    await page.evaluate(() => {
      const original = window.__focusflowResult,
        pending = new Map();
      let next = 800000;
      window.__focusflowResult = (id, r) => {
        if (pending.has(id)) {
          const p = pending.get(id);
          pending.delete(id);
          r.ok ? p.resolve(r.value) : p.reject(Error(r.error));
        } else original(id, r);
      };
      window.syncTest = (cmd, args = {}) =>
        new Promise((resolve, reject) => {
          const id = String(++next);
          pending.set(id, { resolve, reject });
          window.FocusFlowAndroid.request(id, cmd, JSON.stringify(args));
        });
    });
  }
  await attach();
  const native = (cmd, args = {}) =>
    page.evaluate(({ cmd, args }) => window.syncTest(cmd, args), { cmd, args });
  await native("start", {
    mode: "stopwatch",
    kind: "work",
    categoryId: 5,
    minutes: 0,
  });
  await sleep(1500);
  await native("action", { action: "finish" });
  await page.getByRole("button", { name: "Ayarlar", exact: true }).click();
  await page.getByLabel("Bilgisayardaki bağlantı kodu").fill(code);
  await page
    .getByRole("button", { name: "Eşleştir ve eşitle", exact: true })
    .click();
  await page
    .getByText("Cihazlar eşitlendi", { exact: true })
    .waitFor({ timeout: 30000 });
  const a = await native("snapshot"),
    b = await rpc("snapshot");
  assert.equal(a.sessions.length, 2);
  assert.equal(b.sessions.length, 2);
  const total = (s) => s.days.reduce((n, d) => n + d.duration_ms, 0);
  assert.equal(total(a), total(b));
  const expected = total(a);
  results.real_jni_tls_bidirectional_single_total = true;
  for (let i = 0; i < 3; i++) {
    await native("sync_configure", { action: "now" });
    await sleep(1200);
  }
  assert.equal(total(await native("snapshot")), expected);
  assert.equal((await rpc("snapshot")).sessions.length, 2);
  results.repeated_sync_does_not_duplicate = true;
  await rpc("rename");
  await native("sync_configure", { action: "now" });
  await until(
    async () =>
      (await native("snapshot")).categories.some(
        (c) => c.name === "WiFi Physics",
      ),
    "Course change did not arrive",
  );
  results.course_change_propagates = true;
  await assert.rejects(() => native("sync_configure", { action: "host" }));
  await assert.rejects(() =>
    native("sync_configure", {
      action: "connect",
      code: "FF1|8.8.8.8:443|x|x",
    }),
  );
  results.android_cannot_host_or_contact_public_address = true;
  await adb("shell", "am", "force-stop", pkg);
  await sleep(1200);
  await adb(
    "shell",
    "am",
    "start",
    "-n",
    `${pkg}/com.focusflow.mobile.MainActivity`,
  );
  await sleep(2000);
  const pid = await until(
    async () =>
      (
        await adb("shell", "pidof", pkg).catch(() => ({ stdout: "" }))
      ).stdout.trim(),
    "Android did not restart",
  );
  page = await (
    await device.webView(
      { socketName: `webview_devtools_remote_${pid}` },
      { timeout: 60000 },
    )
  ).page();
  await attach();
  await until(
    async () => (await native("sync_status")).last_success,
    "Saved pairing did not reconnect",
  );
  assert.equal(total(await native("snapshot")), expected);
  results.pairing_survives_app_restart = true;
  await native("sync_configure", { action: "off" });
  assert.equal((await native("sync_status")).mode, "off");
  results.disconnect_is_functional = true;
  await page.getByRole("button", { name: "Ayarlar", exact: true }).click();
  await mkdir("android/test-results", { recursive: true });
  await writeFile(
    "android/test-results/wifi-sync.png",
    (
      await exec("adb", ["-s", serial, "exec-out", "screencap", "-p"], {
        encoding: "buffer",
        maxBuffer: 16 * 1024 * 1024,
      })
    ).stdout,
  );
  await writeFile(
    "android/test-results/sync-results.json",
    JSON.stringify(results, null, 2) + "\n",
  );
  console.log("Real Android Wi-Fi sync passed", results);
} finally {
  await rpc("off").catch(() => {});
  host.stdin.end();
  host.kill();
  if (device) await device.close();
}
