// The Android shell around the game: SDL's Java activity and a manifest. The game itself is
// the launcher crate built as a shared library, and SDL's Java half and libSDL2.so come from
// the SDL release the Dockerfile builds - so this only builds through build-android.sh, which
// hands app/build.gradle.kts where all three are.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "dr-rustario-vs-rustris"
include(":app")
