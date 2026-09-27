plugins {
    id("com.android.application")
}

// build-android.sh passes all of these; see Dockerfile.android for where each comes from
fun required(name: String): String =
    providers.gradleProperty(name).orNull
        ?: throw GradleException("-P$name is required: build with build-android.sh")

android {
    namespace = "com.ax_h.drrustariovsrustris"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.ax_h.drrustariovsrustris"
        // the adaptive icon is the only icon, and 26 is where they start
        minSdk = 26
        targetSdk = 36
        versionCode = required("versionCode").toInt()
        versionName = required("versionName")
    }

    sourceSets {
        getByName("main") {
            // SDL's Java half, from the same release as the libSDL2.so beside the game
            java.directories.add(required("sdlJavaDir"))
            // libSDL2.so and liblauncher.so, under the ABI's own folder
            jniLibs.directories.add(required("jniLibsDir"))
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // signed by build-android.sh with a key kept outside the image
            signingConfig = null
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        // lint has nothing to say about SDL's Java that SDL has not already heard
        checkReleaseBuilds = false
    }
}
