package org.bitsr.screenreader

import android.app.Activity
import android.os.Bundle
import android.widget.TextView

/**
 * Settings Activity for bit_sr on Android.
 * Renders the native Slint UI or settings interface.
 */
class SettingsActivity : Activity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val textView = TextView(this).apply {
            text = "bit_sr Screen Reader\n\nUltra-low-latency native accessibility engine initialized."
            textSize = 18f
            setPadding(48, 48, 48, 48)
        }
        setContentView(textView)
    }
}
