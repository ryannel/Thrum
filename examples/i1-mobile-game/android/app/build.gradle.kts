plugins {
    id("com.android.application")
}

android {
    namespace = "org.thrum.i1"
    compileSdk = 35

    defaultConfig {
        applicationId = "org.thrum.i1"
        minSdk = 28
        targetSdk = 35
        versionCode = 1
        versionName = "0.1"
    }

    sourceSets.getByName("main").jniLibs.srcDir("build/generated/jniLibs")

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // android-activity 0.6.1's bundled GameActivity glue targets this API.
    implementation("androidx.games:games-activity:4.4.0")
}
