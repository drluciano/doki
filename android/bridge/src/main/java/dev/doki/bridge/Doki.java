package dev.doki.bridge;

/**
 * Entry point to the Doki engine (the Rust library {@code doki_jni}).
 *
 * <p>Every call takes one JSON request and returns one JSON response, as
 * documented in {@code crates/doki-api/src/lib.rs}. All safety decisions
 * (backups, rechecks, the install journal, recovery) happen inside the
 * engine; Java code only supplies save folders through {@link SaveFolder}.
 *
 * <p>Calls block on file I/O and, for {@code sync}, the network. Run them off
 * the UI thread, one at a time per vault.
 */
public final class Doki {
    static {
        System.loadLibrary("doki_jni");
    }

    private Doki() {}

    /**
     * Run one request.
     *
     * @param vaultPath app-private directory holding Doki's checkpoints
     * @param requestJson a request object with a {@code cmd} field
     * @param opener resolves a game's stored location string to a folder
     * @return {@code {"ok":true,"result":...}} or
     *     {@code {"ok":false,"error":{"code","message","save_may_be_mixed"}}}
     */
    public static native String call(String vaultPath, String requestJson, SaveFolderOpener opener);
}
