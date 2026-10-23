package com.focusflow.mobile;

import org.json.JSONObject;

final class NativeBridge {
    static volatile String language = "tr";
    private static JSONObject messages = new JSONObject();
    static void loadTranslations(android.content.Context context) throws Exception {
        try (java.io.InputStream in = context.getAssets().open("messages.en.json")) {
            java.io.ByteArrayOutputStream bytes = new java.io.ByteArrayOutputStream();
            byte[] buffer = new byte[4096]; int n;
            while ((n=in.read(buffer))!=-1) bytes.write(buffer,0,n);
            messages = new JSONObject(bytes.toString("UTF-8"));
        }
    }
    static String tr(String text) {
        if (!"en".equals(language)) return text;
        if (messages.has(text)) return messages.optString(text,text);
        java.util.ArrayList<String> keys = new java.util.ArrayList<>();
        messages.keys().forEachRemaining(keys::add);
        keys.sort((a,b)->Integer.compare(b.length(),a.length()));
        for (String key:keys) text=text.replace(key,messages.optString(key));
        return text;
    }
    static { System.loadLibrary("focusflow_android"); }
    static native String initialize(String database);
    private static native String call(String command, String args);
    static JSONObject invoke(String command, JSONObject args) throws Exception {
        JSONObject response = new JSONObject(call(command, args.toString()));
        if (!response.optBoolean("ok")) throw new Exception(response.optString("error", "Kayıt hatası"));
        JSONObject value = response.optJSONObject("value");
        if (value != null && value.has("language")) language=value.optString("language","tr");
        if (value != null && value.optJSONObject("settings") != null) language=value.getJSONObject("settings").optString("language","tr");
        if ("save_settings".equals(command)) language=args.getJSONObject("settings").optString("language","tr");
        if ("restore_data".equals(command)) invoke("snapshot");
        return response;
    }
    static JSONObject invoke(String command) throws Exception { return invoke(command, new JSONObject()); }
    static JSONObject tick() throws Exception { return invoke("service_tick").getJSONObject("value"); }
    static void pause() throws Exception { invoke("action", new JSONObject().put("action", "pause")); }
}
