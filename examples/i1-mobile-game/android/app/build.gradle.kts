plugins {
    id("com.android.application")
}

android {
    namespace = "org.thrum.i1"
    compileSdk {
        version = release(37) { minorApiLevel = 2 }
    }
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "org.thrum.i1"
        minSdk = 28
        targetSdk = 35
        versionCode = 1
        versionName = "0.1"
    }

    sourceSets.getByName("main").jniLibs.directories.add("build/generated/jniLibs")

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // android-activity 0.6.1's bundled GameActivity glue targets this API.
    implementation("androidx.games:games-activity:4.4.0")
    // GameActivity's Java superclass exposes this type in its public API.
    implementation("androidx.core:core:1.19.1")
    implementation("androidx.appcompat:appcompat:1.8.0")
}
