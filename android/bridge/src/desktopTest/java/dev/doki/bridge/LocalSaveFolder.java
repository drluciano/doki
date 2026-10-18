package dev.doki.bridge;

import java.io.FileNotFoundException;
import java.io.IOException;
import java.nio.file.*;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.stream.Stream;

/**
 * Desktop {@link SaveFolder} over java.nio, used to test the JNI bridge on a
 * normal JVM. Staged copies are siblings named {@code .doki-<op>-<name>}, the
 * same layout the Android adapter uses. {@code replaceMode} chooses between
 * rename-over and delete-then-rename so both install paths are exercised.
 */
final class LocalSaveFolder implements SaveFolder {
    private final Path root;
    private final int mode;

    LocalSaveFolder(Path root, int mode) {
        this.root = root;
        this.mode = mode;
    }

    private Path file(String path) {
        return root.resolve(path.replace('/', java.io.File.separatorChar));
    }

    private Path staged(String op, String path) {
        Path target = file(path);
        return target.resolveSibling(".doki-" + op + "-" + target.getFileName());
    }

    @Override public String describe() { return root.toString(); }

    @Override public int replaceMode() { return mode; }

    @Override public void checkAvailable() throws IOException {
        if (!Files.isDirectory(root)) throw new FileNotFoundException(root.toString());
    }

    @Override public String[] list() throws IOException {
        checkAvailable();
        List<String> out = new ArrayList<>();
        try (Stream<Path> walk = Files.walk(root)) {
            for (Path p : (Iterable<Path>) walk::iterator) {
                Path rel = root.relativize(p);
                boolean reserved = false;
                for (Path part : rel) reserved |= part.toString().startsWith(".doki-");
                if (!reserved && Files.isRegularFile(p, LinkOption.NOFOLLOW_LINKS)) {
                    out.add(rel.toString().replace(java.io.File.separatorChar, '/'));
                }
            }
        }
        Collections.sort(out);
        return out.toArray(new String[0]);
    }

    @Override public byte[] read(String path) throws IOException {
        try {
            return Files.readAllBytes(file(path));
        } catch (NoSuchFileException e) {
            throw new FileNotFoundException(path);
        }
    }

    @Override public void stage(String op, String path, byte[] data) throws IOException {
        Path s = staged(op, path);
        Files.createDirectories(s.getParent());
        Files.write(s, data);
    }

    @Override public byte[] readStaged(String op, String path) throws IOException {
        return Files.readAllBytes(staged(op, path));
    }

    @Override public void commitStaged(String op, String path) throws IOException {
        Path s = staged(op, path), target = file(path);
        if (mode == REPLACE_DELETE_THEN_RENAME) {
            Files.deleteIfExists(target);
            Files.move(s, target);
        } else {
            Files.move(s, target, StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING);
        }
    }

    @Override public void remove(String path) throws IOException {
        Files.deleteIfExists(file(path));
    }

    @Override public void discardStaging(String op) throws IOException {
        String prefix = ".doki-" + op + "-";
        try (Stream<Path> walk = Files.walk(root)) {
            for (Path p : (Iterable<Path>) walk::iterator) {
                if (p.getFileName().toString().startsWith(prefix)) Files.deleteIfExists(p);
            }
        }
    }
}
