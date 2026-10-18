package dev.doki.app;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.text.DateFormat;
import java.util.Date;
import java.util.HashSet;
import java.util.Set;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

import dev.doki.bridge.Doki;

/**
 * Keeps Doki working while you play in an emulator: every {@link #TICK_SECONDS}
 * it asks the engine to check save folders ({@code auto_check}, which only
 * captures a folder after it has stopped changing), and every
 * {@link #SYNC_EVERY_TICKS} ticks it syncs with paired devices. Neither ever
 * writes a save folder.
 *
 * <p>Android only lets an app keep running in the background as a foreground
 * service with a visible notification, which this is. The user turns it off
 * from the notification or in the app; the choice is kept in
 * {@link #PREFS}.
 */
public class WatchService extends Service {
    static final String PREFS = "doki";
    static final String PREF_ENABLED = "watch_enabled";
    private static final String ACTION_STOP = "dev.doki.app.STOP_WATCHING";
    private static final String CHANNEL_ID = "watching";
    private static final int NOTIFICATION_ID = 1;
    private static final long TICK_SECONDS = 15;
    private static final int SYNC_EVERY_TICKS = 8;

    private ScheduledExecutorService ticker;
    private int ticks;
    private boolean foreground;
    /** Incoming progress already seen; null until the first sync seeds it. */
    private Set<String> seenIncoming;

    /** Start or stop watching and remember the choice. */
    static void setEnabled(Context context, boolean enabled) {
        context.getSharedPreferences(PREFS, MODE_PRIVATE).edit().putBoolean(PREF_ENABLED, enabled).apply();
        Intent intent = new Intent(context, WatchService.class);
        if (!enabled) {
            context.stopService(intent);
        } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(intent);
        } else {
            context.startService(intent);
        }
    }

    static boolean isEnabled(Context context) {
        return context.getSharedPreferences(PREFS, MODE_PRIVATE).getBoolean(PREF_ENABLED, true);
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && ACTION_STOP.equals(intent.getAction())) {
            setEnabled(this, false);
            stopSelf();
            return START_NOT_STICKY;
        }
        showNotification("Watching your saves for new progress");
        if (ticker == null) {
            ticker = Executors.newSingleThreadScheduledExecutor();
            ticker.scheduleWithFixedDelay(this::tick, 0, TICK_SECONDS, TimeUnit.SECONDS);
        }
        return START_STICKY;
    }

    private void tick() {
        try {
            String vault = new File(getFilesDir(), "vault").getAbsolutePath();
            if (!new File(vault).isDirectory()) return; // Not set up yet.
            JSONArray checks = call(vault, "{\"cmd\":\"auto_check\"}").optJSONArray("result");
            JSONObject settings = null;
            for (int i = 0; checks != null && i < checks.length(); i++) {
                JSONObject check = checks.getJSONObject(i);
                String outcome = check.optString("outcome");
                if (!"created".equals(outcome) && !"detected".equals(outcome)) continue;
                if (settings == null) settings = settings(vault);
                String name = check.optString("name");
                String key = "game:" + check.optString("bundle_id");
                if ("created".equals(outcome)) {
                    String time = DateFormat.getTimeInstance(DateFormat.SHORT).format(new Date());
                    showNotification("Checkpoint saved for " + name + " at " + time);
                    if (settings.optBoolean("notify_checkpoints", true)) {
                        Notifier.show(this, key, "Checkpoint saved", name + " is protected up to now.");
                    }
                } else if (settings.optBoolean("notify_changes", true)) {
                    Notifier.show(this, key, "New progress in " + name,
                            "Doki will save a checkpoint once the game stops writing.");
                }
            }
            if (ticks++ % SYNC_EVERY_TICKS == 0) {
                call(vault, "{\"cmd\":\"peer_sync\"}");
                notifyNewIncoming(vault);
            }
        } catch (Throwable t) {
            // A failed tick is retried on the next one; problems with a
            // particular game show in the app.
        }
    }

    private JSONObject call(String vault, String request) throws org.json.JSONException {
        ContentResolver resolver = getContentResolver();
        return new JSONObject(Doki.call(vault, request, location -> SafSaveFolder.open(resolver, location)));
    }

    private JSONObject settings(String vault) throws org.json.JSONException {
        JSONObject result = call(vault, "{\"cmd\":\"settings\"}").optJSONObject("result");
        return result != null ? result : new JSONObject();
    }

    /** Tells the user about progress that arrived since the last sync. */
    private void notifyNewIncoming(String vault) throws org.json.JSONException {
        JSONArray incoming = call(vault, "{\"cmd\":\"incoming\"}").optJSONArray("result");
        if (incoming == null) return;
        Set<String> now = new HashSet<>();
        Set<String> seen = seenIncoming;
        boolean notify = seen != null && settings(vault).optBoolean("notify_incoming", true);
        for (int i = 0; i < incoming.length(); i++) {
            JSONObject proposal = incoming.getJSONObject(i).getJSONObject("proposal");
            String game = proposal.optString("bundle_id");
            String id = game + "." + proposal.optString("checkpoint");
            now.add(id);
            if (notify && !seen.contains(id)) {
                Notifier.show(this, "incoming:" + game, "Progress from " + proposal.optString("from_device"),
                        "New progress is waiting in Incoming. Nothing changes until you choose.");
            }
        }
        seenIncoming = now;
    }

    private synchronized void showNotification(String text) {
        NotificationManager manager = getSystemService(NotificationManager.class);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel channel = new NotificationChannel(
                    CHANNEL_ID, "Watching saves", NotificationManager.IMPORTANCE_LOW);
            channel.setDescription("Shown while Doki checks your saves in the background");
            manager.createNotificationChannel(channel);
        }
        int immutable = Build.VERSION.SDK_INT >= Build.VERSION_CODES.M ? PendingIntent.FLAG_IMMUTABLE : 0;
        PendingIntent open = PendingIntent.getActivity(this, 0,
                new Intent(this, MainActivity.class), immutable);
        PendingIntent stop = PendingIntent.getService(this, 1,
                new Intent(this, WatchService.class).setAction(ACTION_STOP), immutable);
        Notification.Builder builder = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
                ? new Notification.Builder(this, CHANNEL_ID)
                : new Notification.Builder(this);
        Notification notification = builder
                .setSmallIcon(R.mipmap.ic_launcher)
                .setContentTitle("Doki")
                .setContentText(text)
                .setOngoing(true)
                .setContentIntent(open)
                .addAction(0, "Stop watching", stop)
                .build();
        if (foreground) {
            manager.notify(NOTIFICATION_ID, notification);
            return;
        }
        foreground = true;
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE);
            } else {
                startForeground(NOTIFICATION_ID, notification);
            }
        } catch (RuntimeException e) {
            // Android 12+ refuses to start it from the background (for
            // example a restart after the system stopped it). It starts
            // again the next time Doki opens.
            stopSelf();
        }
    }

    @Override
    public void onDestroy() {
        if (ticker != null) ticker.shutdownNow();
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
