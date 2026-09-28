package com.ax_h.drrustariovsrustris;

import org.libsdl.app.SDLActivity;

// SDL loads the last library named here and calls its SDL_main, in launcher/src/android.rs.
public class MainActivity extends SDLActivity {
    @Override
    protected String[] getLibraries() {
        return new String[] { "SDL2", "launcher" };
    }
}
