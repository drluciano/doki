package dev.doki.bridge;

import java.io.IOException;

/**
 * One emulator save folder, as the engine sees it.
 *
 * <p>This mirrors the Rust {@code SaveLocation} trait. Paths are relative,
 * use {@code /} separators and are validated by the engine. The engine calls
 * {@link #commitStaged} and {@link #remove} only inside its journaled install,
 * after a verified backup exists.
 *
 * <p>Errors: throw {@link java.io.FileNotFoundException} when the folder or a
 * file is missing, {@link SecurityException} when access was revoked, and any
 * other {@link IOException} for everything else. Never report a missing
 * folder as empty.
 */
public interface SaveFolder {
    /** Engine does not know how this folder replaces files; installs are refused. */
    int REPLACE_UNKNOWN = 0;
    /** Rename over the old file in one step. */
    int REPLACE_ATOMIC = 1;
    /** Delete the old file, then rename the verified staged copy into place. */
    int REPLACE_DELETE_THEN_RENAME = 2;

    /** Human-readable location for local display. */
    String describe();

    /** One of the {@code REPLACE_*} constants. */
    int replaceMode();

    void checkAvailable() throws IOException;

    /**
     * All regular files of the bundle as relative paths. Must exclude
     * anything whose name starts with {@code .doki-} (Doki's staging).
     */
    String[] list() throws IOException;

    byte[] read(String path) throws IOException;

    /** Write a staged copy for operation {@code op}; never touches playable files. */
    void stage(String op, String path, byte[] data) throws IOException;

    byte[] readStaged(String op, String path) throws IOException;

    /** Replace or create the playable file at {@code path} with the staged copy. */
    void commitStaged(String op, String path) throws IOException;

    /** Remove a playable file. Removing an absent file succeeds. */
    void remove(String path) throws IOException;

    /** Delete everything staged for {@code op}. */
    void discardStaging(String op) throws IOException;
}
