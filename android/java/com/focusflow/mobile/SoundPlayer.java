package com.focusflow.mobile;

import android.content.Context;
import android.media.AudioAttributes;
import android.media.AudioManager;
import android.media.MediaPlayer;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

/** Native, finite playback: works without a visible WebView or autoplay permission. */
final class SoundPlayer {
    interface Result { void complete(String error); }
    private static final Handler main = new Handler(Looper.getMainLooper());
    private static MediaPlayer player;
    private static Result waiting;
    private static volatile boolean playing;
    static boolean isPlaying() { return playing; }
    static void play(Context context, String cue, int volume, Result result) {
        if (!(cue.equals("start") || cue.equals("complete")) || volume < 1 || volume > 100) {
            if (result != null) result.complete(NativeBridge.tr("Geçersiz ses seçimi veya ses düzeyi."));
            return;
        }
        // Mark before posting so completion alarms keep the foreground service alive.
        playing = true;
        Context app = context.getApplicationContext();
        main.post(() -> {
            finish("Ses çalma iptal edildi.");
            try {
                AudioAttributes attributes = new AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_ALARM)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build();
                MediaPlayer created = MediaPlayer.create(app,
                    cue.equals("start") ? R.raw.focusflow_start : R.raw.focusflow_complete,
                    attributes, AudioManager.AUDIO_SESSION_ID_GENERATE);
                if (created == null) throw new IllegalStateException("Ses çalınamadı.");
                player = created; waiting = result; playing = true;
                created.setLooping(false);
                created.setVolume(volume / 100f, volume / 100f);
                created.setOnCompletionListener(p -> {
                    if (player == p) { Log.i("FocusFlowAudio", "played:" + cue); finish(null); }
                });
                created.setOnErrorListener((p, what, extra) -> {
                    if (player == p) finish("Ses çalınamadı.");
                    return true;
                });
                created.start();
                main.postDelayed(() -> { if (player == created) finish("Ses çalma zaman aşımı."); }, 6000);
            } catch (Exception e) {
                Log.w("FocusFlowAudio", "Playback failed", e);
                if (player != null) finish("Ses çalınamadı.");
                else { playing = false; if (result != null) result.complete(NativeBridge.tr("Ses çalınamadı.")); }
            }
        });
    }
    private static void finish(String error) {
        MediaPlayer old = player; Result callback = waiting;
        player = null; waiting = null; playing = false;
        if (old != null) { try { old.release(); } catch (Exception ignored) { } }
        if (callback != null) callback.complete(error == null ? null : NativeBridge.tr(error));
    }
}
