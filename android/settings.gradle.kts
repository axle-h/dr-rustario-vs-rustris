// SDL's activity and a manifest around the launcher library; builds only through
// build-android.sh, which passes where the game and SDL's libraries are.
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
