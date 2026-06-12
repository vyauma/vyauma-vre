package org.vyauma.vre.app;

import android.app.Activity;
import android.os.Bundle;

public class MainActivity extends Activity {
    static {
        System.loadLibrary("vre_core");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // Start VRE VM
        runVreApp("app.vym");
    }

    private native void runVreApp(String assetPath);
}
