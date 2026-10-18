package dev.doki.safprobe;

import android.app.Activity;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.os.Handler;
import android.os.Looper;
import android.provider.DocumentsContract;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.ScrollView;
import android.view.WindowInsets;
import android.graphics.Insets;

/** Research harness: immediate-child metadata only; never opens file contents. */
public final class MainActivity extends Activity {
    private static final int PICK = 1;
    private TextView result;
    private Button choose;
    private final Handler main = new Handler(Looper.getMainLooper());
    private volatile boolean destroyed;
    private CancellationSignal active;
    private Runnable activeTimeout;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        int padding = (int) (16 * getResources().getDisplayMetrics().density);
        layout.setPadding(padding, padding, padding, padding);
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            layout.setOnApplyWindowInsetsListener((view, windowInsets) -> {
                Insets bars = windowInsets.getInsets(WindowInsets.Type.systemBars()
                        | WindowInsets.Type.displayCutout());
                view.setPadding(padding + bars.left, padding + bars.top,
                        padding + bars.right, padding + bars.bottom);
                return windowInsets;
            });
        }
        result = new TextView(this);
        result.setText("Research only. Select a SMALL disposable fixture folder, never a save root. "
                + "Reads metadata for at most 20 immediate children. No contents, names, IDs, paths or network. "
                + "The provider may still enumerate the entire selected folder internally. "
                + "No support or backup guarantee.");
        choose = new Button(this);
        choose.setText("Choose disposable folder (read-only)");
        choose.setOnClickListener(v -> {
            Intent request = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
            request.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                    | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
            try { startActivityForResult(request, PICK); }
            catch (RuntimeException ex) { result.setText("System picker unavailable. No data inspected."); }
        });
        layout.addView(result);
        layout.addView(choose);
        Button forget = new Button(this);
        forget.setText("Release probe's remembered grants");
        forget.setOnClickListener(v -> {
            try {
                for (android.content.UriPermission p : getContentResolver().getPersistedUriPermissions()) {
                    int flags = (p.isReadPermission() ? Intent.FLAG_GRANT_READ_URI_PERMISSION : 0)
                            | (p.isWritePermission() ? Intent.FLAG_GRANT_WRITE_URI_PERMISSION : 0);
                    getContentResolver().releasePersistableUriPermission(p.getUri(), flags);
                }
                result.setText("Remembered grants released. This did not modify selected files.");
            } catch (RuntimeException ex) { result.setText("Could not release all grants. Use Android app settings."); }
        });
        layout.addView(forget);
        ScrollView scroll = new ScrollView(this);
        scroll.addView(layout);
        setContentView(scroll);
        if (state != null) result.setText(state.getString("report", "Previous inspection interrupted; choose fixture again."));
        layout.requestApplyInsets();
    }

    @Override protected void onSaveInstanceState(Bundle state) {
        state.putString("report", result.getText().toString());
        super.onSaveInstanceState(state);
    }

    @Override protected void onDestroy() {
        destroyed = true;
        if (active != null) active.cancel();
        if (activeTimeout != null) main.removeCallbacks(activeTimeout);
        super.onDestroy();
    }

    @Override protected void onActivityResult(int code, int status, Intent data) {
        super.onActivityResult(code, status, data);
        if (code != PICK) return;
        if (status != RESULT_OK || data == null || data.getData() == null) {
            result.setText("Selection cancelled. No inspection performed."); return;
        }
        final Uri tree = data.getData();
        if (!"content".equals(tree.getScheme())
                || (data.getFlags() & Intent.FLAG_GRANT_READ_URI_PERMISSION) == 0) {
            result.setText("No usable read grant. No inspection performed."); return;
        }
        boolean persisted = false;
        if ((data.getFlags() & Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION) != 0) {
            try {
                getContentResolver().takePersistableUriPermission(tree, Intent.FLAG_GRANT_READ_URI_PERMISSION);
                for (android.content.UriPermission p : getContentResolver().getPersistedUriPermissions()) {
                    if (p.getUri().equals(tree) && p.isReadPermission()) persisted = true;
                }
            } catch (RuntimeException ignored) { /* Outcome shown without exception paths/IDs. */ }
        }
        final boolean remembered = persisted;
        choose.setEnabled(false);
        final CancellationSignal cancel = new CancellationSignal();
        active = cancel;
        result.setText("Inspecting metadata. Cancellation requested after five seconds; provider may ignore it. Rotation cancels this attempt; select again.");
        final Runnable timeout = () -> {
            cancel.cancel();
            if (!destroyed) result.setText("Timeout: cancellation requested. Provider may still be running. No contents opened; wait or close the probe.");
        };
        activeTimeout = timeout;
        main.postDelayed(timeout, 5000);
        new Thread(() -> {
            String message;
            try {
                Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(
                        tree, DocumentsContract.getTreeDocumentId(tree));
                String[] columns = {DocumentsContract.Document.COLUMN_MIME_TYPE,
                        DocumentsContract.Document.COLUMN_FLAGS};
                int count = 0, directories = 0, writeAdvertised = 0;
                boolean more;
                try (Cursor cursor = getContentResolver().query(children, columns, null, null, null, cancel)) {
                    if (cursor == null) throw new IllegalStateException();
                    while (count < 20 && cursor.moveToNext()) {
                        cancel.throwIfCanceled();
                        count++;
                        if (DocumentsContract.Document.MIME_TYPE_DIR.equals(cursor.getString(0))) directories++;
                        if ((cursor.getInt(1) & DocumentsContract.Document.FLAG_SUPPORTS_WRITE) != 0) writeAdvertised++;
                    }
                    more = cursor.moveToNext();
                }
                message = "Read grant: yes. Persisted read grant observed: " + remembered
                        + ". Immediate entries sampled: " + count + "; directories: " + directories
                        + "; file-write flags advertised: " + writeAdvertised + "; additional entries: " + more
                        + ". Advertised write capability was NOT exercised. No contents opened. "
                        + "Persistence after reboot and device compatibility are untested.";
            } catch (RuntimeException ex) {
                message = "Metadata inspection failed or timed out. Persisted read grant observed: "
                        + remembered + ". No contents opened. Provider errors are redacted.";
            }
            final String output = message;
            if (!destroyed) main.post(() -> {
                main.removeCallbacks(timeout);
                if (destroyed) return;
                active = null;
                activeTimeout = null;
                result.setText(output);
                choose.setEnabled(true);
            });
        }, "metadata-probe").start();
    }
}
