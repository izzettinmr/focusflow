package com.focusflow.mobile;

import org.json.JSONObject;

final class NativeBridge {
    static { System.loadLibrary("focusflow_android"); }
    static native String initialize(String database);
    private static native String call(String command, String args);
    static JSONObject invoke(String command, JSONObject args) throws Exception {
        JSONObject response = new JSONObject(call(command, args.toString()));
        if (!response.optBoolean("ok")) throw new Exception(response.optString("error", "Kayıt hatası"));
        return response;
    }
    static JSONObject invoke(String command) throws Exception { return invoke(command, new JSONObject()); }
    static JSONObject tick() throws Exception { return invoke("service_tick").getJSONObject("value"); }
    static void pause() throws Exception { invoke("action", new JSONObject().put("action", "pause")); }
}
