plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "dev.doki.app"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "dev.doki.app"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    sourceSets {
        getByName("main") {
            // Java side of the engine bridge (dev.doki.bridge), shared with the
            // desktop JVM test. libdoki_jni.so comes from src/main/jniLibs,
            // built by tools/build-android-engine.sh.
            java.srcDir("../../../android/bridge/src/main/java")
        }
    }

    // Release builds are signed with Doki's own key when these are set (CI
    // sets them from repository secrets; see docs/release.md). Every APK
    // signed with the same key installs over the previous one and keeps the
    // app's data. Without them, release builds fall back to the debug key so
    // `flutter build apk` keeps working on any machine, but such an APK cannot
    // update one signed with Doki's key, or the other way round.
    val keystorePath = System.getenv("DOKI_ANDROID_KEYSTORE")
    val releaseSigned = !keystorePath.isNullOrEmpty()
    signingConfigs {
        if (releaseSigned) {
            create("release") {
                storeFile = file(keystorePath!!)
                storePassword = System.getenv("DOKI_ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("DOKI_ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("DOKI_ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName(if (releaseSigned) "release" else "debug")
            // Keeps the bridge names the native engine calls (see the file).
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
