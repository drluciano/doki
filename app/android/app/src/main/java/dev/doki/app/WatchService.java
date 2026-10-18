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
            ContentResolver resolver = getContentResolver();
            if (!new File(vault).isDirectory()) return; // Not set up yet.
            JSONObject response = new JSONObject(Doki.call(vault, "{\"cmd\":\"auto_check\"}",
                    location -> SafSaveFolder.open(resolver, location)));
            JSONArray checks = response.optJSONArray("result");
            for (int i = 0; checks != null && i < checks.length(); i++) {
                JSONObject check = checks.getJSONObject(i);
                if ("created".equals(check.optString("outcome"))) {
                    showNotification("Checkpoint saved for " + check.optString("name") + " at "
                            + DateFormat.getTimeInstance(DateFormat.SHORT).format(new Date()));
                }
            }
            if (ticks++ % SYNC_EVERY_TICKS == 0) {
                Doki.call(vault, "{\"cmd\":\"peer_sync\"}", location -> SafSaveFolder.open(resolver, location));
            }
        } catch (Throwable t) {
            // A failed tick is retried on the next one; problems with a
            // particular game show in the app.
        }
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
