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
let csp;
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
    if (csp) res.setHeader("content-security-policy", csp);
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
const out = path.resolve(
  root,
  process.env.FOCUSFLOW_AUDIT_OUTPUT || "audit/2026-10-22",
);
await mkdir(out, { recursive: true });
let canaryRequests = 0;
const canary = createServer((req, res) => {
  canaryRequests++;
  res.end("audit-only");
});
await new Promise((resolve) => canary.listen(0, "127.0.0.1", resolve));
const canaryUrl = `http://127.0.0.1:${canary.address().port}/canary`;
const policy = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"))
  .app.security.csp;
// Taşıyıcının aynı origin IPC isteği için tek ek izin; canary ayrı porttadır.
csp = policy.replace(
  "connect-src ",
  `connect-src http://127.0.0.1:${server.address().port} `,
);
try {
  browser = await chromium.launch({
    headless: true,
    executablePath: process.env.FOCUSFLOW_BROWSER,
  });
  const page = await browser.newPage({
    viewport: { width: 1280, height: 900 },
  });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(String(e)));
  await page.addInitScript(() => {
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
    window.__auditPwned = false;
    window.__cspViolations = [];
    document.addEventListener("securitypolicyviolation", (e) =>
      window.__cspViolations.push({
        directive: e.effectiveDirective,
        blocked: e.blockedURI,
      }),
    );
  });
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  await page.getByRole("heading", { name: "Genel bakış" }).waitFor();
  const payload = '<img src=x onerror="window.__auditPwned=true">';
  assert(
    !(
      "error" in
      (await rpc("save_category", {
        id: null,
        name: payload,
        color: "#aabbcc",
      }))
    ),
  );
  await page.waitForTimeout(1200);
  await page.getByRole("button", { name: "Derslerim", exact: true }).click();
  await page.getByRole("heading", { name: payload, exact: true }).waitFor();
  assert.equal(await page.evaluate(() => window.__auditPwned), false);
  assert.equal(await page.locator("img[src=x]").count(), 0);
  const cspChecks = await page.evaluate(async (url) => {
    let fetchBlocked = false;
    try {
      await fetch(url);
    } catch {
      fetchBlocked = true;
    }
    const image = new Image();
    const imageBlocked = await new Promise((resolve) => {
      image.onload = () => resolve(false);
      image.onerror = () => resolve(true);
      image.src = url + "?image";
    });
    const script = document.createElement("script");
    script.textContent = "window.__auditPwned=true";
    document.body.append(script);
    script.remove();
    const object = document.createElement("object");
    object.data = url + "?object";
    document.body.append(object);
    await new Promise((r) => setTimeout(r, 100));
    object.remove();
    return {
      fetchBlocked,
      imageBlocked,
      inlineScriptExecuted: window.__auditPwned,
      violations: window.__cspViolations,
    };
  }, canaryUrl);
  assert(
    cspChecks.fetchBlocked &&
      cspChecks.imageBlocked &&
      !cspChecks.inlineScriptExecuted,
  );
  assert.equal(canaryRequests, 0);
  await page.getByRole("button", { name: "Genel bakış", exact: true }).click();
  const session = await page.context().newCDPSession(page);
  await session.send("HeapProfiler.enable");
  await session.send("Performance.enable");
  async function metric(label) {
    await page
      .getByRole("button", { name: "Genel bakış", exact: true })
      .click();
    await page.waitForTimeout(150);
    await session.send("HeapProfiler.collectGarbage");
    await page.waitForTimeout(150);
    const { metrics } = await session.send("Performance.getMetrics");
    const counters = await session.send("Memory.getDOMCounters");
    return {
      label,
      heap_used_bytes: metrics.find((m) => m.name === "JSHeapUsedSize").value,
      ...counters,
    };
  }
  const points = [];
  const screens = [
    "Odak zamanı",
    "İstatistikler",
    "Çalışma geçmişi",
    "Derslerim",
    "Genel bakış",
  ];
  // Isınma ardından her 200 geçişte aynı ekranda GC sonrası karşılaştırma.
  const batches = Number(process.env.AUDIT_BATCHES || 7),
    batchSize = Number(process.env.AUDIT_BATCH_SIZE || 200);
  for (let batch = 0; batch < batches; batch++) {
    for (let i = 0; i < batchSize; i++) {
      const name = screens[i % screens.length];
      await page.evaluate((name) => {
        const b = [...document.querySelectorAll("aside nav button")].find(
          (b) => b.textContent.trim() === name,
        );
        if (!b) throw new Error(name);
        b.click();
      }, name);
      await page.evaluate(() => new Promise(requestAnimationFrame));
    }
    points.push(
      await metric(
        batch === 0 ? "warmup" : `after_${batch * batchSize}_changes`,
      ),
    );
  }
  await page.waitForTimeout(10000);
  points.push(await metric("settled"));
  assert.deepEqual(pageErrors, []);
  const result = {
    test_engine: "Chromium + gerçek Rust/SQLite test taşıyıcısı",
    navigation_changes: batches * batchSize,
    comparison_warmup_changes: batchSize,
    xss_escaped: true,
    csp: cspChecks,
    canary_requests: canaryRequests,
    points,
    page_errors: pageErrors,
    temp_database: temp,
  };
  await readFile(path.join(temp, "test.db")).catch(() => null);
  await import("node:fs/promises").then((fs) =>
    fs.writeFile(
      path.join(out, "renderer-security-memory.json"),
      JSON.stringify(result, null, 2),
    ),
  );
  console.log(JSON.stringify(result, null, 2));
} finally {
  if (browser) await browser.close();
  child.stdin.end();
  server.close();
  canary.close();
}
