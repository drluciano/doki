package dev.doki.app;

import android.Manifest;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;

/**
 * Notifications about events: a checkpoint saved, a save folder that
 * started changing, progress that arrived from another device. Used by both
 * the app (through the "notify" channel method) and {@link WatchService}.
 *
 * <p>Each notification has a key naming what it is about, such as
 * {@code game:<id>}, so a newer one about the same game replaces the older
 * one instead of piling up. They go to their own channel, separate from the
 * quiet ongoing "Watching saves" notification, so the user can tune them in
 * Android's settings. Nothing is shown when notifications are not allowed.
 */
final class Notifier {
    private static final String CHANNEL_ID = "events";
    /** Ids below this are reserved for the ongoing notification. */
    private static final int FIRST_ID = 1000;

    private Notifier() {}

    static void show(Context context, String key, String title, String body) {
        if (Build.VERSION.SDK_INT >= 33
                && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
                        != PackageManager.PERMISSION_GRANTED) {
            return;
        }
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel channel = new NotificationChannel(
                    CHANNEL_ID, "Checkpoints and progress", NotificationManager.IMPORTANCE_DEFAULT);
            channel.setDescription(
                    "Checkpoints Doki saved, saves that changed, and progress from your other devices");
            manager.createNotificationChannel(channel);
        }
        int immutable = Build.VERSION.SDK_INT >= Build.VERSION_CODES.M ? PendingIntent.FLAG_IMMUTABLE : 0;
        PendingIntent open = PendingIntent.getActivity(context, 0,
                new Intent(context, MainActivity.class), immutable);
        Notification.Builder builder = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
                ? new Notification.Builder(context, CHANNEL_ID)
                : new Notification.Builder(context);
        Notification notification = builder
                .setSmallIcon(R.mipmap.ic_launcher)
                .setContentTitle(title)
                .setContentText(body)
                .setStyle(new Notification.BigTextStyle().bigText(body))
                .setContentIntent(open)
                .setAutoCancel(true)
                .build();
        manager.notify(FIRST_ID + Math.floorMod(key.hashCode(), 1_000_000), notification);
    }
}
