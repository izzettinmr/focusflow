package com.focusflow.mobile;

import android.Manifest;
import android.app.*;
import android.content.*;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.net.Uri;
import android.os.*;
import android.view.*;
import android.webkit.*;
import android.widget.FrameLayout;
import org.json.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.*;
import java.util.concurrent.*;

public final class MainActivity extends Activity {
    private static final String ORIGIN = "https://appassets.androidplatform.net";
    private static final int EXPORT = 71, IMPORT = 72, NOTIFICATIONS = 73;
    private static final int MAX_BACKUP = 100_000_000;
    private final ExecutorService io = Executors.newSingleThreadExecutor();
    private final Set<String> commands = new HashSet<>(Arrays.asList("snapshot", "start", "action", "save_settings", "save_category", "archive_category", "edit_session", "delete_session", "data_path", "hide_window", "quit", "export_file", "restore_file", "open_app_settings", "sync_status", "sync_configure"));
    private WebView web;
    private String fileRequest, exportData, waitingStartId, waitingStartArgs;
    private boolean destroyed = false;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(Color.rgb(16,17,25));
        web = new WebView(this);
        web.setBackgroundColor(Color.rgb(16,17,25));
        root.addView(web, new FrameLayout.LayoutParams(-1,-1));
        setContentView(root);
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().setDecorFitsSystemWindows(false);
            root.setOnApplyWindowInsetsListener((v, insets) -> {
                android.graphics.Insets bars = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
                android.graphics.Insets ime = insets.getInsets(WindowInsets.Type.ime());
                v.setPadding(bars.left,bars.top,bars.right,Math.max(bars.bottom,ime.bottom));
                return WindowInsets.CONSUMED;
            });
        }
        if (Build.VERSION.SDK_INT >= 33) getOnBackInvokedDispatcher().registerOnBackInvokedCallback(0, () -> moveTaskToBack(true));
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(false);
        s.setAllowFileAccess(false);
        s.setAllowContentAccess(false);
        s.setAllowFileAccessFromFileURLs(false);
        s.setAllowUniversalAccessFromFileURLs(false);
        s.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
        s.setSupportMultipleWindows(false);
        s.setJavaScriptCanOpenWindowsAutomatically(false);
        WebView.setWebContentsDebuggingEnabled((getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0);
        CookieManager.getInstance().setAcceptCookie(false);
        web.setWebViewClient(new WebViewClient() {
            @Override public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) { return true; }
            @Override public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest request) {
                Uri u = request.getUrl();
                if (!"https".equals(u.getScheme()) || !"appassets.androidplatform.net".equals(u.getHost()) || u.getPort() != -1 || !"GET".equals(request.getMethod())) return blocked();
                String path = u.getPath();
                if (path == null || path.contains("..") || path.contains("\\") || path.indexOf('\0') >= 0) return blocked();
                if (path.equals("/")) path = "/index.html";
                String mime = path.endsWith(".js") ? "text/javascript" : path.endsWith(".css") ? "text/css" : path.endsWith(".html") ? "text/html" : path.endsWith(".svg") ? "image/svg+xml" : path.endsWith(".png") ? "image/png" : "application/octet-stream";
                try {
                    Map<String,String> headers = new HashMap<>();
                    headers.put("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'none'; object-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none'");
                    headers.put("X-Content-Type-Options", "nosniff");
                    headers.put("Cache-Control", "no-store");
                    return new WebResourceResponse(mime,"UTF-8",200,"OK",headers,getAssets().open("web" + path));
                } catch (IOException e) { return blocked(); }
            }
        });
        web.addJavascriptInterface(new AndroidApi(), "FocusFlowAndroid");
        web.loadUrl(ORIGIN + "/index.html");
    }
    private WebResourceResponse blocked() {
        return new WebResourceResponse("text/plain","UTF-8",403,"Forbidden",Collections.emptyMap(),new ByteArrayInputStream(new byte[0]));
    }
    private void reply(String id, JSONObject value) {
        runOnUiThread(() -> { if (!destroyed) web.evaluateJavascript("window.__focusflowResult && window.__focusflowResult(" + JSONObject.quote(id) + "," + value.toString() + ");", null); });
    }
    private void success(String id, Object v) {
        try { reply(id,new JSONObject().put("ok",true).put("value",v == null ? JSONObject.NULL : v)); } catch (JSONException ignored) { }
    }
    private void error(String id, Exception e) {
        try { reply(id,new JSONObject().put("ok",false).put("error",e.getMessage() == null ? NativeBridge.tr("İşlem tamamlanamadı") : e.getMessage())); } catch (JSONException ignored) { }
    }
    private void syncTimer() throws Exception {
        JSONObject state = NativeBridge.tick();
        TimerService.showMessages(this,state);
        JSONObject active = state.optJSONObject("active");
        if (active != null && "running".equals(active.optString("status"))) startForegroundService(new Intent(this,TimerService.class));
        else { TimerService.cancelAlarm(this); stopService(new Intent(this,TimerService.class)); }
    }
    private void execute(String id, String cmd, String args) {
        io.execute(() -> {
            try {
                JSONObject result = NativeBridge.invoke(cmd,new JSONObject(args));
                if (cmd.equals("start") || cmd.equals("action")) {
                    try { syncTimer(); }
                    catch (Exception e) { NativeBridge.pause(); throw e; }
                }
                reply(id,result);
            } catch (Exception e) { error(id,e); }
        });
    }
    private final class AndroidApi {
        @JavascriptInterface public void request(String id, String cmd, String args) {
            if (id == null || !id.matches("[0-9]{1,16}")) return;
            if (cmd == null || !commands.contains(cmd) || args == null || args.length()>1_000_000) { error(id,new Exception(NativeBridge.tr("Geçersiz komut"))); return; }
            runOnUiThread(() -> {
                if (destroyed) return;
                if (cmd.equals("open_app_settings")) {
                    try { startActivity(new Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS,Uri.parse("package:" + getPackageName()))); success(id,null); }
                    catch (Exception e) { error(id,e); } return;
                }
                if (cmd.equals("export_file") || cmd.equals("restore_file")) { pickFile(id,cmd,args); return; }
                if (cmd.equals("hide_window")) { moveTaskToBack(true); success(id,null); return; }
                if (cmd.equals("quit")) {
                    io.execute(() -> { try { NativeBridge.pause(); syncTimer(); success(id,null); runOnUiThread(() -> moveTaskToBack(true)); } catch (Exception e) { error(id,e); } }); return;
                }
                if (cmd.equals("start") && Build.VERSION.SDK_INT >= 33 && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                    waitingStartId=id; waitingStartArgs=args;
                    requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS},NOTIFICATIONS); return;
                }
                execute(id,cmd,args);
            });
        }
    }
    @Override public void onRequestPermissionsResult(int code, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(code,permissions,results);
        if (code==NOTIFICATIONS && waitingStartId!=null) {
            String id=waitingStartId, args=waitingStartArgs; waitingStartId=null; waitingStartArgs=null;
            // Android permits the FGS without this grant; completion alerts need the grant.
            if (results.length==0 || results[0]!=PackageManager.PERMISSION_GRANTED)
                android.widget.Toast.makeText(this,NativeBridge.tr("Zamanlayıcı çalışır; bitiş uyarıları için Ayarlar → Telefon bölümünden bildirim izni verin."),android.widget.Toast.LENGTH_LONG).show();
            execute(id,"start",args);
        }
    }
    private void pickFile(String id, String cmd, String args) {
        if (fileRequest!=null) { error(id,new Exception(NativeBridge.tr("Önce açık dosya işlemini tamamlayın."))); return; }
        fileRequest=id;
        if (cmd.equals("export_file")) {
            io.execute(() -> {
                try {
                    String format=new JSONObject(args).getString("format");
                    exportData=NativeBridge.invoke("export_data",new JSONObject().put("format",format)).getString("value");
                    runOnUiThread(() -> {
                        Intent i=new Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType(format.equals("json") ? "application/json" : "text/csv");
                        i.putExtra(Intent.EXTRA_TITLE,"FocusFlow-" + new SimpleDateFormat("yyyy-MM-dd",Locale.ROOT).format(new Date()) + "." + format);
                        try { startActivityForResult(i,EXPORT); } catch (Exception e) { fileRequest=null; exportData=null; error(id,e); }
                    });
                } catch (Exception e) { runOnUiThread(() -> { fileRequest=null; exportData=null; error(id,e); }); }
            });
        } else {
            Intent i=new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*");
            i.putExtra(Intent.EXTRA_MIME_TYPES,new String[]{"application/json","text/plain","application/octet-stream"});
            try { startActivityForResult(i,IMPORT); } catch (Exception e) { fileRequest=null; error(id,e); }
        }
    }
    @Override protected void onActivityResult(int request, int result, Intent intent) {
        super.onActivityResult(request,result,intent);
        if (request!=EXPORT && request!=IMPORT) return;
        final String id=fileRequest; fileRequest=null;
        if (id==null) return;
        if (result!=RESULT_OK || intent==null || intent.getData()==null) { exportData=null; success(id,false); return; }
        Uri uri=intent.getData();
        if (!"content".equals(uri.getScheme())) { exportData=null; error(id,new Exception(NativeBridge.tr("Android dosya sağlayıcısı gerekli."))); return; }
        if (request==EXPORT) {
            final String data=exportData; exportData=null;
            io.execute(() -> {
                try (OutputStream out=getContentResolver().openOutputStream(uri,"wt")) {
                    if (out==null || data==null) throw new IOException(NativeBridge.tr("Dosya açılamadı"));
                    out.write(data.getBytes(StandardCharsets.UTF_8)); out.flush(); success(id,true);
                } catch (Exception e) { error(id,e); }
            });
        } else io.execute(() -> {
            try {
                String raw;
                try (InputStream in=getContentResolver().openInputStream(uri); ByteArrayOutputStream out=new ByteArrayOutputStream()) {
                    if (in==null) throw new IOException(NativeBridge.tr("Yedek açılamadı"));
                    byte[] buf=new byte[16384]; int n, total=0;
                    while ((n=in.read(buf))!=-1) { total+=n; if (total>MAX_BACKUP) throw new IOException(NativeBridge.tr("Yedek 100 MB sınırını aşıyor.")); out.write(buf,0,n); }
                    raw=new String(out.toByteArray(),StandardCharsets.UTF_8);
                }
                JSONObject args=new JSONObject().put("data",raw);
                NativeBridge.invoke("validate_restore",args);
                runOnUiThread(() -> {
                    if (destroyed) return;
                    new AlertDialog.Builder(this).setTitle(NativeBridge.tr("Yedeği geri yükle"))
                        .setMessage(NativeBridge.tr("Mevcut kayıtlar bu yedekle değiştirilecek. Önce mevcut kayıtların otomatik güvenlik yedeği alınacak. Bilgisayar ve telefon kayıtları otomatik birleşmez."))
                        .setNegativeButton(NativeBridge.tr("Vazgeç"),(d,w)->success(id,false))
                        .setOnCancelListener(d->success(id,false))
                        .setPositiveButton(NativeBridge.tr("Geri yükle"),(d,w)->io.execute(()-> { try { NativeBridge.invoke("restore_data",args); success(id,true); } catch (Exception e) { error(id,e); } })).show();
                });
            } catch (Exception e) { error(id,e); }
        });
    }
    @Override public void onBackPressed() { moveTaskToBack(true); }
    @Override protected void onDestroy() {
        destroyed=true;
        web.removeJavascriptInterface("FocusFlowAndroid"); web.destroy();
        io.shutdown(); super.onDestroy();
    }
}
