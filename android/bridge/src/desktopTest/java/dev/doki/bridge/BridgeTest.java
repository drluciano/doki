package dev.doki.bridge;

import java.io.FileNotFoundException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * End-to-end check of the JNI bridge on a desktop JVM: capture, restore an
 * older checkpoint through review + use (with both replace modes), and
 * error mapping for a missing folder and an unknown replace mode.
 * Run by {@code tools/jni-desktop-test.sh}; exits non-zero on failure.
 */
public final class BridgeTest {
    private static int mode = SaveFolder.REPLACE_ATOMIC;

    private static final SaveFolderOpener OPENER = location -> {
        if (location.endsWith("missing")) throw new FileNotFoundException(location);
        return new LocalSaveFolder(Path.of(location), mode);
    };

    private static String call(Path vault, String json) {
        String out = Doki.call(vault.toString(), json, OPENER);
        System.out.println(json + "\n  -> " + out);
        return out;
    }

    private static String ok(Path vault, String json) {
        String out = call(vault, json);
        check(out.startsWith("{\"ok\":true"), "expected ok: " + out);
        return out;
    }

    private static String field(String json, String name) {
        Matcher m = Pattern.compile("\"" + name + "\":\"([^\"]*)\"").matcher(json);
        check(m.find(), "no " + name + " in " + json);
        return m.group(1);
    }

    private static void check(boolean cond, String msg) {
        if (!cond) throw new AssertionError(msg);
    }

    private static String quote(Path p) {
        return "\"" + p.toString().replace("\\", "\\\\") + "\"";
    }

    private static String restoreRoundTrip(Path tmp, String name, int replaceMode) throws Exception {
        mode = replaceMode;
        Path vault = tmp.resolve("vault-" + name), save = tmp.resolve("save-" + name);
        Files.createDirectories(save.resolve("sub"));
        ok(vault, "{\"cmd\":\"init\",\"name\":\"Desk\"}");
        ok(vault, "{\"cmd\":\"add\",\"name\":\"" + name + "\",\"location\":" + quote(save) + "}");
        Files.writeString(save.resolve("game.sav"), "one");
        Files.writeString(save.resolve("sub/extra.dat"), "keep");
        String first = field(ok(vault, "{\"cmd\":\"capture\",\"game\":\"" + name + "\"}"), "checkpoint");
        Files.writeString(save.resolve("game.sav"), "two");
        Files.delete(save.resolve("sub/extra.dat"));
        ok(vault, "{\"cmd\":\"capture\",\"game\":\"" + name + "\"}");

        String review = ok(vault, "{\"cmd\":\"review\",\"game\":\"" + name + "\",\"checkpoint\":\""
                + first.substring(0, 12) + "\"}");
        String reviewObj = review.substring(review.indexOf("\"result\":") + 9, review.length() - 1);
        ok(vault, "{\"cmd\":\"use\",\"review\":" + reviewObj + "}");
        check(Files.readString(save.resolve("game.sav")).equals("one"), "game.sav not restored");
        check(Files.readString(save.resolve("sub/extra.dat")).equals("keep"), "extra.dat not restored");
        try (var s = Files.list(save.resolve("sub"))) {
            check(s.noneMatch(p -> p.getFileName().toString().startsWith(".doki-")), "staging left behind");
        }
        String history = ok(vault, "{\"cmd\":\"history\",\"game\":\"" + name + "\"}");
        check(history.contains("Restored checkpoint"), "no restore choice in history");
        check(ok(vault, "{\"cmd\":\"recovery_status\",\"game\":\"" + name + "\"}").contains("\"result\":null"),
                "recovery pending");
        return first;
    }

    public static void main(String[] args) throws Exception {
        Path tmp = Files.createTempDirectory("doki-bridge");
        String first = restoreRoundTrip(tmp, "Atomic", SaveFolder.REPLACE_ATOMIC);
        restoreRoundTrip(tmp, "Saf", SaveFolder.REPLACE_DELETE_THEN_RENAME);

        // An unknown replace mode refuses to install and leaves files alone.
        mode = SaveFolder.REPLACE_UNKNOWN;
        Path vault = tmp.resolve("vault-Atomic"), save = tmp.resolve("save-Atomic");
        Files.writeString(save.resolve("game.sav"), "three");
        String review = ok(vault, "{\"cmd\":\"review\",\"game\":\"Atomic\",\"checkpoint\":\"" + first + "\"}");
        String reviewObj = review.substring(review.indexOf("\"result\":") + 9, review.length() - 1);
        String refused = call(vault, "{\"cmd\":\"use\",\"review\":" + reviewObj + "}");
        check(refused.contains("\"code\":\"StrategyUnsupported\""), "unknown mode not refused: " + refused);
        check(Files.readString(save.resolve("game.sav")).equals("three"), "files touched on refusal");

        // A folder Java cannot open is reported, never treated as empty.
        Path v2 = tmp.resolve("vault-missing");
        ok(v2, "{\"cmd\":\"init\",\"name\":\"Desk\"}");
        String gone = call(v2, "{\"cmd\":\"add\",\"name\":\"Gone\",\"location\":\"/nowhere/missing\"}");
        check(gone.contains("\"code\":\"SourceUnavailable\""), "missing folder not reported: " + gone);

        String bad = call(v2, "not json");
        check(bad.contains("\"code\":\"InvalidInput\""), "bad request not rejected");
        System.out.println("bridge test passed");
    }
}
