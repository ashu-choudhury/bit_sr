package org.bitsr.screenreader

/**
 * High-performance, zero-allocation native JNI bridge connecting
 * Android AccessibilityService and input events directly to the Rust engine.
 */
object NativeBridge {
    init {
        System.loadLibrary("bit_sr_platform_android")
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
}
