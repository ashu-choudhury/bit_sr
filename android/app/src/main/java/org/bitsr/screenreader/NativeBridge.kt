package org.bitsr.screenreader

import android.util.Log

/**
 * High-performance, zero-allocation native JNI bridge connecting
 * Android AccessibilityService and input events directly to the unified Rust engine (`libbit_sr.so`).
 */
object NativeBridge {
    private const val TAG = "BitSrNativeBridge"

    init {
        try {
            System.loadLibrary("bit_sr")
            Log.i(TAG, "Successfully loaded libbit_sr.so")
        } catch (e: UnsatisfiedLinkError) {
            try {
                System.loadLibrary("bit_sr_platform_android")
                Log.i(TAG, "Successfully loaded fallback libbit_sr_platform_android.so")
            } catch (fallbackError: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load native screen reader library", fallbackError)
            }
        }
    }

    /**
     * Callback interface implemented by BitSrAccessibilityService to receive
     * audio and action requests from the native Rust engine.
     */
    interface HostCallback {
        fun onSpeakText(text: String, interrupt: Boolean)
        fun onPerformAction(actionId: Int, targetNodeId: Long): Boolean
    }

    @Volatile
    var hostCallback: HostCallback? = null

    /**
     * Invoked from native Rust engine (`AndroidTtsDriver`) to speak formatted text or interrupt speech.
     */
    @JvmStatic
    fun speakText(text: String, interrupt: Boolean) {
        hostCallback?.onSpeakText(text, interrupt)
    }

    /**
     * Invoked from native Rust engine (`AndroidActionPerformer`) to perform accessibility actions.
     */
    @JvmStatic
    fun performAction(actionId: Int, targetNodeId: Long): Boolean {
        return hostCallback?.onPerformAction(actionId, targetNodeId) ?: false
    }

    /**
     * Initializes the Rust engine coordinator and spatial hit-testing system.
     * Returns a 64-bit raw pointer to the Rust Engine instance.
     */
    external fun initEngine(): Long

    /**
     * Shuts down and frees the native engine coordinator.
     */
    external fun destroyEngine(enginePtr: Long)

    /**
     * Ultra-fast raw physical touch injection.
     * Zero Java objects are created or allocated.
     *
     * @param enginePtr Native engine pointer
     * @param action MotionEvent actionMasked (ACTION_DOWN=0, ACTION_UP=1, ACTION_MOVE=2, etc.)
     * @param x Physical X screen coordinate in pixels
     * @param y Physical Y screen coordinate in pixels
     * @param eventTime Milliseconds timestamp of the hardware event
     * @return true if the event was consumed by the screen reader, false to pass through
     */
    external fun onRawTouch(
        enginePtr: Long,
        action: Int,
        x: Float,
        y: Float,
        eventTime: Long
    ): Boolean

    /**
     * Fast event notification when window hierarchy changes or focus shifts.
     */
    external fun onAccessibilityEvent(
        enginePtr: Long,
        eventType: Int,
        packageName: String,
        className: String,
        text: String,
        contentDescription: String,
        nodeSourceId: Long,
        left: Int,
        top: Int,
        right: Int,
        bottom: Int
    )

    /**
     * Caches a batch of accessible nodes harvested from the active window hierarchy.
     */
    external fun updateWindowTree(
        enginePtr: Long,
        windowId: Int,
        nodeIds: LongArray,
        roles: IntArray,
        boundsLeft: IntArray,
        boundsTop: IntArray,
        boundsRight: IntArray,
        boundsBottom: IntArray,
        texts: Array<String>,
        states: LongArray
    )

    /**
     * Caches the entire screen snapshot across all foreground/system windows.
     */
    external fun updateEntireScreen(
        enginePtr: Long,
        nodeIds: LongArray,
        roles: IntArray,
        boundsLeft: IntArray,
        boundsTop: IntArray,
        boundsRight: IntArray,
        boundsBottom: IntArray,
        texts: Array<String>,
        states: LongArray
    )

    /**
     * Configures the double-tap detection window in ms and spatial tolerance in px.
     */
    external fun setDoubleTapConfig(
        enginePtr: Long,
        timeoutMs: Long,
        distancePx: Float
    )
}
