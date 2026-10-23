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
        try { NativeBridge.loadTranslations(this); } catch (Exception e) { throw new IllegalStateException(e); }
        NotificationManager nm = getSystemService(NotificationManager.class);
        nm.createNotificationChannel(new NotificationChannel(TIMER_CHANNEL, NativeBridge.tr("Aktif çalışma"), NotificationManager.IMPORTANCE_LOW));
        nm.createNotificationChannel(new NotificationChannel(DONE_CHANNEL, NativeBridge.tr("Tamamlama ve hedefler"), NotificationManager.IMPORTANCE_DEFAULT));
        try {
            JSONObject response = new JSONObject(NativeBridge.initialize(new java.io.File(getFilesDir(), "focusflow.db").getAbsolutePath()));
            if (!response.optBoolean("ok")) throw new IllegalStateException(response.optString("error"));
            NativeBridge.invoke("snapshot");
            nm.createNotificationChannel(new NotificationChannel(TIMER_CHANNEL, NativeBridge.tr("Aktif çalışma"), NotificationManager.IMPORTANCE_LOW));
            nm.createNotificationChannel(new NotificationChannel(DONE_CHANNEL, NativeBridge.tr("Tamamlama ve hedefler"), NotificationManager.IMPORTANCE_DEFAULT));
        } catch (Exception e) { throw new IllegalStateException(NativeBridge.tr("Çalışma kayıtları açılamadı"), e); }
    }
}
