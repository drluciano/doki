package dev.doki.app;

import android.content.ContentResolver;
import android.content.UriPermission;
import android.database.Cursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.DocumentsContract.Document;

import java.io.ByteArrayOutputStream;
import java.io.FileNotFoundException;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.SyncFailedException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

import dev.doki.bridge.SaveFolder;

/**
 * A save folder reached through the Storage Access Framework (a document
 * tree the user picked, with a persisted read/write grant).
 *
 * <p>Document providers cannot rename over an existing file, so replacement is
 * {@link SaveFolder#REPLACE_DELETE_THEN_RENAME}: the engine stages and verifies
 * a new sibling document named {@code .doki-<op>-<name>}, then this class
 * deletes the old file and renames the staged one into place. An interruption
 * between the two leaves the file missing, never half written, and the
 * engine's journal and vault copies recover it. Files are never overwritten in
 * place, so providers that ignore truncation cannot corrupt a save.
 *
 * <p>Every precondition (rename and delete support, the provider keeping the
 * exact name) is checked before the old file is deleted.
 */
final class SafSaveFolder implements SaveFolder {
    private static final String[] COLUMNS = {
        Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME,
        Document.COLUMN_MIME_TYPE, Document.COLUMN_FLAGS,
    };

    private static final class Doc {
        final String id;
        final String name;
        final boolean dir;
        final int flags;

        Doc(String id, String name, String mime, int flags) {
            this.id = id;
            this.name = name;
            this.dir = Document.MIME_TYPE_DIR.equals(mime);
            this.flags = flags;
        }
    }

    private final ContentResolver resolver;
    private final Uri tree;
    private final String rootId;

    private SafSaveFolder(ContentResolver resolver, Uri tree) {
        this.resolver = resolver;
        this.tree = tree;
        this.rootId = DocumentsContract.getTreeDocumentId(tree);
    }

    /** Open a folder from the location string the engine stores (a tree URI). */
    static SafSaveFolder open(ContentResolver resolver, String location) throws IOException {
        Uri uri = Uri.parse(location);
        if (!DocumentsContract.isTreeUri(uri)) {
            throw new FileNotFoundException("not a folder chosen in Doki: " + location);
        }
        return new SafSaveFolder(resolver, uri);
    }

    /** Folder name to show the user, or the URI when the provider has none. */
    static String displayName(ContentResolver resolver, Uri tree) {
        try {
            return new SafSaveFolder(resolver, tree).root().name;
        } catch (Exception e) {
            return tree.toString();
        }
    }

    private Uri uri(String id) {
        return DocumentsContract.buildDocumentUriUsingTree(tree, id);
    }

    private static Doc read(Cursor c) {
        return new Doc(c.getString(0), c.getString(1), c.getString(2), c.getInt(3));
    }

    private Doc query(Uri uri) throws IOException {
        try (Cursor c = resolver.query(uri, COLUMNS, null, null, null)) {
            if (c == null || !c.moveToFirst()) throw new FileNotFoundException(uri.toString());
            return read(c);
        } catch (IllegalArgumentException e) {
            throw new FileNotFoundException(uri + ": " + e.getMessage());
        }
    }

    private Doc root() throws IOException {
        return query(uri(rootId));
    }

    private List<Doc> children(Doc parent) throws IOException {
        Uri u = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parent.id);
        List<Doc> out = new ArrayList<>();
        try (Cursor c = resolver.query(u, COLUMNS, null, null, null)) {
            if (c == null) throw new FileNotFoundException(u.toString());
            while (c.moveToNext()) out.add(read(c));
        } catch (IllegalArgumentException e) {
            throw new FileNotFoundException(u + ": " + e.getMessage());
        }
        return out;
    }

    private Doc child(Doc parent, String name) throws IOException {
        for (Doc d : children(parent)) {
            if (d.name.equals(name)) return d;
        }
        return null;
    }

    /** Parent directory of {@code path}, optionally creating missing ones. */
    private Doc parentOf(String path, boolean create) throws IOException {
        Doc dir = root();
        String[] parts = path.split("/");
        for (int i = 0; i < parts.length - 1; i++) {
            Doc next = child(dir, parts[i]);
            if (next == null) {
                if (!create) return null;
                Uri made = DocumentsContract.createDocument(
                        resolver, uri(dir.id), Document.MIME_TYPE_DIR, parts[i]);
                if (made == null) throw new IOException("could not create folder " + parts[i]);
                next = query(made);
                if (!next.name.equals(parts[i])) {
                    throw new IOException("storage renamed folder " + parts[i] + " to " + next.name);
                }
            } else if (!next.dir) {
                throw new IOException(parts[i] + " is a file, expected a folder");
            }
            dir = next;
        }
        return dir;
    }

    private static String leaf(String path) {
        return path.substring(path.lastIndexOf('/') + 1);
    }

    private static String stagedName(String op, String path) {
        return ".doki-" + op + "-" + leaf(path);
    }

    private Doc find(String path) throws IOException {
        Doc parent = parentOf(path, false);
        return parent == null ? null : child(parent, leaf(path));
    }

    private Doc findStaged(String op, String path) throws IOException {
        Doc parent = parentOf(path, false);
        return parent == null ? null : child(parent, stagedName(op, path));
    }

    private byte[] readAll(Doc d) throws IOException {
        try (InputStream in = resolver.openInputStream(uri(d.id))) {
            if (in == null) throw new FileNotFoundException(d.name);
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buf = new byte[64 * 1024];
            for (int n; (n = in.read(buf)) > 0; ) out.write(buf, 0, n);
            return out.toByteArray();
        }
    }

    private void delete(Doc d) throws IOException {
        if ((d.flags & Document.FLAG_SUPPORTS_DELETE) == 0) {
            throw new IOException("storage does not allow deleting " + d.name);
        }
        if (!DocumentsContract.deleteDocument(resolver, uri(d.id))) {
            throw new IOException("could not delete " + d.name);
        }
    }

    @Override
    public String describe() {
        try {
            return root().name;
        } catch (Exception e) {
            return tree.toString();
        }
    }

    @Override
    public int replaceMode() {
        try {
            return (root().flags & Document.FLAG_DIR_SUPPORTS_CREATE) != 0
                    ? REPLACE_DELETE_THEN_RENAME
                    : REPLACE_UNKNOWN;
        } catch (Exception e) {
            return REPLACE_UNKNOWN;
        }
    }

    @Override
    public void checkAvailable() throws IOException {
        boolean granted = false;
        for (UriPermission p : resolver.getPersistedUriPermissions()) {
            if (p.getUri().equals(tree) && p.isReadPermission() && p.isWritePermission()) {
                granted = true;
            }
        }
        if (!granted) throw new SecurityException("access to this folder was not kept");
        if (!root().dir) throw new FileNotFoundException("not a folder");
    }

    @Override
    public String[] list() throws IOException {
        List<String> out = new ArrayList<>();
        walk(root(), "", out);
        Collections.sort(out);
        return out.toArray(new String[0]);
    }

    @Override
    public String[] listUnder(String dir) throws IOException {
        Doc folder = root();
        for (String part : dir.split("/")) {
            folder = child(folder, part);
            if (folder == null || !folder.dir) return new String[0];
        }
        List<String> out = new ArrayList<>();
        walk(folder, dir + "/", out);
        Collections.sort(out);
        return out.toArray(new String[0]);
    }

    @Override
    public String[] entries() throws IOException {
        List<String> out = new ArrayList<>();
        for (Doc d : children(root())) {
            if (d.name.startsWith(".doki-")) continue;
            out.add(d.dir ? d.name + "/" : d.name);
        }
        Collections.sort(out);
        return out.toArray(new String[0]);
    }

    private void walk(Doc dir, String prefix, List<String> out) throws IOException {
        for (Doc d : children(dir)) {
            if (d.name.startsWith(".doki-")) continue;
            if (d.dir) walk(d, prefix + d.name + "/", out);
            else out.add(prefix + d.name);
        }
    }

    @Override
    public byte[] read(String path) throws IOException {
        Doc d = find(path);
        if (d == null) throw new FileNotFoundException(path);
        return readAll(d);
    }

    @Override
    public void stage(String op, String path, byte[] data) throws IOException {
        Doc parent = parentOf(path, true);
        String name = stagedName(op, path);
        Doc old = child(parent, name);
        if (old != null) delete(old);
        Uri made = DocumentsContract.createDocument(
                resolver, uri(parent.id), "application/octet-stream", name);
        if (made == null) throw new IOException("could not create " + name);
        Doc created = query(made);
        if (!created.name.equals(name)) {
            DocumentsContract.deleteDocument(resolver, made);
            throw new IOException("storage renamed " + name + " to " + created.name);
        }
        try (ParcelFileDescriptor pfd = resolver.openFileDescriptor(made, "w")) {
            if (pfd == null) throw new IOException("could not write " + name);
            try (FileOutputStream out = new FileOutputStream(pfd.getFileDescriptor())) {
                out.write(data);
                out.flush();
                try {
                    pfd.getFileDescriptor().sync();
                } catch (SyncFailedException e) {
                    // Some providers hand out pipes. The engine re-reads and
                    // verifies the staged bytes before replacing anything.
                }
            }
        }
    }

    @Override
    public byte[] readStaged(String op, String path) throws IOException {
        Doc d = findStaged(op, path);
        if (d == null) throw new FileNotFoundException(stagedName(op, path));
        return readAll(d);
    }

    @Override
    public void commitStaged(String op, String path) throws IOException {
        Doc staged = findStaged(op, path);
        if (staged == null) throw new FileNotFoundException(stagedName(op, path));
        if ((staged.flags & Document.FLAG_SUPPORTS_RENAME) == 0) {
            throw new IOException("storage does not allow renaming files");
        }
        Doc target = find(path);
        String name = leaf(path);
        if (target != null) delete(target);
        Uri renamed = DocumentsContract.renameDocument(resolver, uri(staged.id), name);
        Doc placed = query(renamed != null ? renamed : uri(staged.id));
        if (!placed.name.equals(name)) {
            throw new IOException("storage named the file " + placed.name + " instead of " + name);
        }
    }

    @Override
    public void remove(String path) throws IOException {
        Doc d = find(path);
        if (d != null) delete(d);
    }

    @Override
    public void discardStaging(String op) throws IOException {
        discard(root(), ".doki-" + op + "-");
    }

    private void discard(Doc dir, String prefix) throws IOException {
        for (Doc d : children(dir)) {
            if (d.dir) discard(d, prefix);
            else if (d.name.startsWith(prefix)) delete(d);
        }
    }
}
