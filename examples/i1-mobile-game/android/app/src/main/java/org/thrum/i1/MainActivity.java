package org.thrum.i1;

import com.google.androidgamesdk.GameActivity;

public final class MainActivity extends GameActivity {
    static {
        System.loadLibrary("i1_mobile_game");
    }

    private static native void nativeSetForeground(boolean active);

    @Override
    protected void onResume() {
        super.onResume();
        nativeSetForeground(true);
    }

    @Override
    protected void onPause() {
        nativeSetForeground(false);
        super.onPause();
    }
}
