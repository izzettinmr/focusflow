package com.focusflow.mobile;

import android.app.Application;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import org.json.JSONObject;

public final class FocusFlowApp extends Application {
    static final String TIMER_CHANNEL = "focusflow_timer";
    static final String DONE_CHANNEL = "focusflow_done";
    @Override public void onCreate() {
        super.onCreate();
        NotificationManager nm = getSystemService(NotificationManager.class);
        nm.createNotificationChannel(new NotificationChannel(TIMER_CHANNEL, "Aktif çalışma", NotificationManager.IMPORTANCE_LOW));
        nm.createNotificationChannel(new NotificationChannel(DONE_CHANNEL, "Tamamlama ve hedefler", NotificationManager.IMPORTANCE_DEFAULT));
        try {
            JSONObject response = new JSONObject(NativeBridge.initialize(new java.io.File(getFilesDir(), "focusflow.db").getAbsolutePath()));
            if (!response.optBoolean("ok")) throw new IllegalStateException(response.optString("error"));
        } catch (Exception e) { throw new IllegalStateException("Çalışma kayıtları açılamadı", e); }
    }
}
