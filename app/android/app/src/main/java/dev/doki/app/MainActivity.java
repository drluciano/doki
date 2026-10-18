package dev.doki.app;

import android.Manifest;
import android.app.Activity;
import android.content.ContentResolver;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;

import androidx.annotation.NonNull;

import java.io.File;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

import dev.doki.bridge.Doki;
import io.flutter.embedding.android.FlutterActivity;
import io.flutter.embedding.engine.FlutterEngine;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;

/**
 * Hosts the Flutter UI and answers its {@code dev.doki/engine} channel:
 * {@code call} runs one engine request (see {@code crates/doki-api}) on a
 * background thread, one at a time per lane: requests that wait on other
 * devices ({@code lane: "network"}) get their own thread so a slow network
 * never holds up the rest of the app; {@code pickFolder}
 * asks the system folder picker for a save folder and keeps the grant;
 * {@code setWatching} and {@code isWatching} control {@link WatchService}.
 */
public class MainActivity extends FlutterActivity {
    private static final String CHANNEL = "dev.doki/engine";
    private static final int PICK_FOLDER = 4201;
    private static final int ASK_NOTIFY = 4202;

    private final ExecutorService engine = Executors.newSingleThreadExecutor();
    private final ExecutorService network = Executors.newSingleThreadExecutor();
    private final Handler main = new Handler(Looper.getMainLooper());
    private MethodChannel.Result pendingPick;

    @Override
    public void configureFlutterEngine(@NonNull FlutterEngine flutterEngine) {
        super.configureFlutterEngine(flutterEngine);
        new MethodChannel(flutterEngine.getDartExecutor().getBinaryMessenger(), CHANNEL)
                .setMethodCallHandler(this::onCall);
    }

    private void onCall(MethodCall call, MethodChannel.Result result) {
        switch (call.method) {
            case "call": {
                String request = call.argument("request");
                String vault = new File(getFilesDir(), "vault").getAbsolutePath();
                ContentResolver resolver = getContentResolver();
                ExecutorService lane = "network".equals(call.argument("lane")) ? network : engine;
                lane.execute(() -> {
                    String response =
                            Doki.call(vault, request, location -> SafSaveFolder.open(resolver, location));
                    main.post(() -> result.success(response));
                });
                break;
            }
            case "setWatching": {
                boolean enabled = Boolean.TRUE.equals(call.argument("enabled"));
                if (enabled) askToNotify();
                WatchService.setEnabled(this, enabled);
                result.success(null);
                break;
            }
            case "notify":
                Notifier.show(this, call.argument("key"), call.argument("title"), call.argument("body"));
                result.success(null);
                break;
            case "isWatching":
                result.success(WatchService.isEnabled(this));
                break;
            case "pickFolder": {
                if (pendingPick != null) {
                    result.error("Busy", "A folder picker is already open", null);
                    return;
                }
                pendingPick = result;
                Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
                intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                        | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                        | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION
                        | Intent.FLAG_GRANT_PREFIX_URI_PERMISSION);
                startActivityForResult(intent, PICK_FOLDER);
                break;
            }
            default:
                result.notImplemented();
        }
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // Notifications tell you about checkpoints and incoming progress.
        askToNotify();
        // Keep checking saves while an emulator is in front, unless turned off.
        if (WatchService.isEnabled(this)) {
            WatchService.setEnabled(this, true);
        }
    }

    /** Android 13+ shows no notifications unless allowed. */
    private void askToNotify() {
        if (Build.VERSION.SDK_INT >= 33
                && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
                        != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS}, ASK_NOTIFY);
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != PICK_FOLDER || pendingPick == null) return;
        MethodChannel.Result result = pendingPick;
        pendingPick = null;
        Uri tree = data == null ? null : data.getData();
        if (resultCode != Activity.RESULT_OK || tree == null) {
            result.success(null); // Cancelled: not an error.
            return;
        }
        try {
            getContentResolver().takePersistableUriPermission(tree,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        } catch (SecurityException e) {
            result.error("PermissionLost", "Android did not allow keeping access to this folder", null);
            return;
        }
        Map<String, String> picked = new HashMap<>();
        picked.put("location", tree.toString());
        picked.put("name", SafSaveFolder.displayName(getContentResolver(), tree));
        result.success(picked);
    }

    @Override
    protected void onDestroy() {
        engine.shutdown();
        super.onDestroy();
    }
}
