package com.focusflow.mobile;

import android.app.*;
import android.content.*;
import android.os.*;
import org.json.*;
import java.util.concurrent.*;

public final class TimerService extends Service {
    static final int NOTIFICATION_ID = 11;
    private ScheduledExecutorService worker;
    private long shownSession = -1, shownTarget = -1;
    private String shownStatus = "";
    private static PendingIntent openApp(Context c) {
        return PendingIntent.getActivity(c, 1, new Intent(c, MainActivity.class), PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }
    private Notification notification(JSONObject active) {
        boolean countdown = active != null && active.optLong("target_ms") > 0;
        long elapsed = active == null ? 0 : active.optLong("elapsed_ms");
        long remaining = active == null ? 0 : Math.max(0, active.optLong("target_ms") - elapsed);
        Notification.Builder b = new Notification.Builder(this, FocusFlowApp.TIMER_CHANNEL)
            .setSmallIcon(com.focusflow.mobile.R.drawable.ic_notification)
            .setContentTitle(active == null ? "FocusFlow" : countdown ? "Pomodoro · Odak zamanı" : "Kronometre · Odak zamanı")
            .setContentText("Çalışman güvenle kaydediliyor. Açmak için dokun.")
            .setContentIntent(openApp(this)).setOngoing(true).setOnlyAlertOnce(true)
            .setCategory(Notification.CATEGORY_SERVICE).setShowWhen(true)
            .setWhen(System.currentTimeMillis() + (countdown ? remaining : -elapsed))
            .setUsesChronometer(true).setChronometerCountDown(countdown);
        return b.build();
    }
    static void showMessages(Context c, JSONObject state) {
        JSONArray messages = state.optJSONArray("messages");
        if (messages == null) return;
        NotificationManager nm = c.getSystemService(NotificationManager.class);
        for (int i=0; i<messages.length(); i++) {
            Notification n = new Notification.Builder(c, FocusFlowApp.DONE_CHANNEL)
                .setSmallIcon(com.focusflow.mobile.R.drawable.ic_notification).setContentTitle("FocusFlow")
                .setContentText(messages.optString(i)).setStyle(new Notification.BigTextStyle().bigText(messages.optString(i)))
                .setContentIntent(openApp(c)).setAutoCancel(true).build();
            nm.notify(100 + i, n);
        }
    }
    static PendingIntent alarmIntent(Context c) {
        return PendingIntent.getBroadcast(c, 20, new Intent(c, TimerAlarm.class), PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }
    static void cancelAlarm(Context c) { c.getSystemService(AlarmManager.class).cancel(alarmIntent(c)); }
    private void schedule(JSONObject active) {
        cancelAlarm(this);
        long remaining = active.optLong("target_ms") - active.optLong("elapsed_ms");
        if (active.optLong("target_ms") <= 0 || remaining <= 0) return;
        AlarmManager am = getSystemService(AlarmManager.class);
        long when = SystemClock.elapsedRealtime() + remaining;
        if (Build.VERSION.SDK_INT < 31 || am.canScheduleExactAlarms())
            am.setExactAndAllowWhileIdle(AlarmManager.ELAPSED_REALTIME_WAKEUP, when, alarmIntent(this));
        else am.setAndAllowWhileIdle(AlarmManager.ELAPSED_REALTIME_WAKEUP, when, alarmIntent(this));
    }
    private synchronized void update() {
        try {
            JSONObject state = NativeBridge.tick();
            showMessages(this, state);
            JSONObject active = state.optJSONObject("active");
            if (active == null || !"running".equals(active.optString("status"))) {
                cancelAlarm(this); stopForeground(STOP_FOREGROUND_REMOVE); stopSelf(); return;
            }
            if (shownSession != active.optLong("id") || shownTarget != active.optLong("target_ms") || !shownStatus.equals(active.optString("status"))) {
                shownSession = active.optLong("id"); shownTarget = active.optLong("target_ms"); shownStatus = active.optString("status");
                getSystemService(NotificationManager.class).notify(NOTIFICATION_ID, notification(active));
                schedule(active);
            }
        } catch (Exception e) {
            // A recording error must stop the timer; never silently continue showing success.
            try { NativeBridge.pause(); } catch (Exception ignored) { }
            JSONObject error = new JSONObject();
            try { error.put("messages", new JSONArray().put("Kayıt hatası: " + e.getMessage())); } catch (JSONException ignored) { }
            showMessages(this, error); cancelAlarm(this); stopForeground(STOP_FOREGROUND_REMOVE); stopSelf();
        }
    }
    @Override public void onCreate() {
        super.onCreate();
        startForeground(NOTIFICATION_ID, notification(null));
        worker = Executors.newSingleThreadScheduledExecutor();
        worker.scheduleWithFixedDelay(this::update, 0, 5, TimeUnit.SECONDS);
    }
    @Override public int onStartCommand(Intent intent, int flags, int id) {
        // Rebuild the chronometer/alarm after a pause/resume in the same service instance.
        shownSession = -1;
        worker.execute(this::update);
        return START_NOT_STICKY;
    }
    @Override public void onDestroy() { if (worker != null) worker.shutdownNow(); super.onDestroy(); }
    @Override public IBinder onBind(Intent intent) { return null; }
}
