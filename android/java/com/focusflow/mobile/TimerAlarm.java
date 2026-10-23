package com.focusflow.mobile;

import android.app.NotificationManager;
import android.content.*;
import org.json.JSONObject;

public final class TimerAlarm extends BroadcastReceiver {
    @Override public void onReceive(Context c, Intent intent) {
        PendingResult pending = goAsync();
        new Thread(() -> {
            try {
                JSONObject state = NativeBridge.tick();
                TimerService.showMessages(c, state);
                JSONObject active = state.optJSONObject("active");
                if (active == null || !"running".equals(active.optString("status"))) {
                    c.stopService(new Intent(c, TimerService.class));
                    c.getSystemService(NotificationManager.class).cancel(TimerService.NOTIFICATION_ID);
                }
            } catch (Exception ignored) { /* Durable Engine recovery pauses if Android killed the process. */ }
            finally { pending.finish(); }
        }, "FocusFlow-alarm").start();
    }
}
