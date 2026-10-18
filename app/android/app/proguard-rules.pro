# The Rust engine (crates/doki-jni) calls these classes and methods by name
# through JNI. R8 cannot see native callers, so without these rules it
# renames them in release builds and every save-folder call fails with
# NoSuchMethodError. tools/check-apk-bridge.sh checks the built APK.
-keep class dev.doki.bridge.** { *; }
-keep class * implements dev.doki.bridge.SaveFolder { *; }
-keep class * implements dev.doki.bridge.SaveFolderOpener { *; }
