// Actual Android Storage Access Framework, native confirmation and SQLite restore.
import { _android } from "playwright";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { writeFile } from "node:fs/promises";
import assert from "node:assert/strict";
const exec = promisify(execFile),
  serial = "emulator-5556",
  pkg = "com.focusflow.mobile.audit";
const adb = (...args) =>
  exec("adb", ["-s", serial, ...args], {
    timeout: 60000,
    maxBuffer: 4 * 1024 * 1024,
  });
const delay = (ms) => new Promise((r) => setTimeout(r, ms));
const q = (s) => "'" + s.replaceAll("'", "'\\''") + "'";
let device;
const results = {};
async function tap(test, perform = true) {
  for (let i = 0; i < 12; i++) {
    await adb(
      "shell",
      "uiautomator",
      "dump",
      "/sdcard/focusflow-test-window.xml",
    );
    const xml = (
      await adb("exec-out", "cat", "/sdcard/focusflow-test-window.xml")
    ).stdout;
    await writeFile("android/test-results/file-picker.xml", xml);
    const nodes = [...xml.matchAll(/<node\b([^>]+)>/g)].map((m) =>
      Object.fromEntries(
        [...m[1].matchAll(/([\w-]+)="([^"]*)"/g)].map((a) => [a[1], a[2]]),
      ),
    );
    const fixtureAnr = nodes.some((n) =>
      /^(System UI|Pixel Launcher).*responding/.test(n.text),
    );
    if (fixtureAnr) {
      const close = nodes.find((n) => n.text === "Close app");
      if (close) {
        const b = close.bounds.match(/\d+/g).map(Number);
        await adb(
          "shell",
          "input",
          "tap",
          String((b[0] + b[2]) / 2),
          String((b[1] + b[3]) / 2),
        );
        console.log(
          "Dismissed emulator system dialog; FocusFlow was not the ANR process.",
        );
        continue;
      }
    }
    const n = nodes.find((n) => n.enabled === "true" && test(n));
    if (n) {
      if (!perform) return n;
      const b = n.bounds.match(/\d+/g).map(Number);
      await adb(
        "shell",
        "input",
        "tap",
        String(Math.round((b[0] + b[2]) / 2)),
        String(Math.round((b[1] + b[3]) / 2)),
      );
      return;
    }
    await delay(500);
  }
  throw Error("Native dialog control not found; see file-picker.xml");
}
try {
  assert(
    (await adb("emu", "avd", "name")).stdout.includes("FocusFlowAudit20261022"),
  );
  await adb("shell", "input", "keyevent", "KEYCODE_WAKEUP");
  await adb("shell", "wm", "dismiss-keyguard");
  await adb("shell", "input", "keyevent", "KEYCODE_BACK");
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
  const page = await (await device.webView({ pkg }, { timeout: 60000 })).page();
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
    window.filesTest = (cmd, args = {}) =>
      new Promise((resolve, reject) => {
        const id = String(++next);
        pending.set(id, { resolve, reject });
        window.FocusFlowAndroid.request(id, cmd, JSON.stringify(args));
      });
  });
  const native = (cmd, args = {}) =>
    page.evaluate(({ cmd, args }) => window.filesTest(cmd, args), {
      cmd,
      args,
    });
  const current = await native("snapshot");
  if (current.sessions.some((s) => ["running", "paused"].includes(s.status)))
    await native("action", { action: "finish" });
  const before = await native("snapshot");
  console.log("Testing native JSON save picker…");
  const exporting = native("export_file", { format: "json" });
  exporting.catch(() => {});
  await tap((n) => /^save$/i.test(n.text));
  assert.equal(await exporting, true);
  const names = (await adb("shell", "ls", "-t", "/sdcard/Download")).stdout
    .trim()
    .split("\n");
  const name = names.find((n) => /^FocusFlow-.*\.json$/.test(n));
  assert(name);
  const raw = (await adb("shell", `cat ${q("/sdcard/Download/" + name)}`))
    .stdout;
  const backup = JSON.parse(raw);
  assert.equal(backup.sessions.length, before.sessions.length);
  results.json_export_real_document_provider = true;
  await native("save_category", {
    id: null,
    name: "Android Dosya Testi " + Date.now(),
    color: "#abcdef",
  });
  console.log("Testing native JSON import and confirmation…");
  const importing = native("restore_file");
  importing.catch(() => {});
  await tap((n) => n.text === name);
  await tap(
    (n) =>
      n["resource-id"] === "android:id/button1" &&
      n.text.toLocaleLowerCase("tr").endsWith("yükle"),
  );
  assert.equal(await importing, true);
  const after = await native("snapshot");
  assert.equal(after.sessions.length, before.sessions.length);
  assert.equal(after.categories.length, before.categories.length);
  const privateFiles = (await adb("shell", "run-as", pkg, "ls", "files"))
    .stdout;
  assert(privateFiles.includes("before-restore-"));
  results.restore_confirmation_safety_backup_and_same_data = true;
  console.log("Testing native CSV save picker…");
  const cancelling = native("export_file", { format: "csv" });
  cancelling.catch(() => {});
  await tap((n) => /^save$/i.test(n.text));
  assert.equal(await cancelling, true);
  const csv = (await adb("shell", "ls", "/sdcard/Download")).stdout
    .split("\n")
    .find((n) => /^FocusFlow-.*\.csv$/.test(n));
  assert(csv);
  const csvRaw = (await adb("shell", `cat ${q("/sdcard/Download/" + csv)}`))
    .stdout;
  assert(csvRaw.includes("süre_saniye"));
  results.csv_export = true;
  const cancel = native("export_file", { format: "json" });
  cancel.catch(() => {});
  await tap((n) => /^save$/i.test(n.text), false);
  await adb("shell", "input", "keyevent", "KEYCODE_BACK");
  assert.equal(await cancel, false);
  results.cancel_returns_false = true;
  await writeFile(
    "android/test-results/native-file-results.json",
    JSON.stringify(results, null, 2),
  );
  console.log("Native file tests passed", results);
} finally {
  if (device) await device.close();
}
