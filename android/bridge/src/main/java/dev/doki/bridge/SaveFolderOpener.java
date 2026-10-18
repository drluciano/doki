package dev.doki.bridge;

import java.io.IOException;

/** Resolves the location string stored for a game to a {@link SaveFolder}. */
public interface SaveFolderOpener {
    /**
     * @throws java.io.FileNotFoundException the folder no longer exists
     * @throws SecurityException access to the folder was revoked
     */
    SaveFolder open(String location) throws IOException;
}
