package org.bitsr.screenreader

import android.accessibilityservice.AccessibilityService
import android.graphics.Rect
import android.os.Build
import android.speech.tts.TextToSpeech
import android.util.Log
import android.view.Display
import android.view.MotionEvent
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import java.util.Locale

/**
 * Ultra-thin AccessibilityService host for bit_sr.
 *
 * All exploration logic, double-tap detection, spatial hit-testing,
 * and audio feedback are executed directly within the native Rust engine.
 */
class BitSrAccessibilityService : AccessibilityService(), NativeBridge.HostCallback, TextToSpeech.OnInitListener {

    private var enginePtr: Long = 0L
    private val tempRect = Rect()
    private var tts: TextToSpeech? = null
    private var isTtsReady: Boolean = false

    companion object {
        private const val TAG = "BitSrService"
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        Log.i(TAG, "Connecting bit_sr AccessibilityService and initializing Rust engine...")

        // Register host callbacks for audio output and action execution
        NativeBridge.hostCallback = this

        // Initialize Android Text-to-Speech engine
        try {
            tts = TextToSpeech(applicationContext, this)
        } catch (t: Throwable) {
            Log.e(TAG, "Failed to initialize Android TextToSpeech", t)
        }

        try {
            enginePtr = NativeBridge.initEngine()
            Log.i(TAG, "bit_sr native engine initialized at address: 0x" + java.lang.Long.toHexString(enginePtr))
        } catch (t: Throwable) {
            Log.e(TAG, "Failed to load bit_sr native library", t)
        }

        // On Android 13+ (API 33+), register with TouchInteractionController for direct raw touchscreen streams
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            try {
                val controller = getTouchInteractionController(Display.DEFAULT_DISPLAY)
                controller.registerCallback(
                    mainExecutor,
                    object : android.accessibilityservice.TouchInteractionController.Callback {
                        override fun onMotionEvent(event: MotionEvent) {
                            handleRawMotionEvent(event)
                        }

                        override fun onStateChanged(state: Int) {
                            // Touch interaction state transitions
                        }
                    }
                )
                Log.i(TAG, "Registered direct TouchInteractionController callback")
            } catch (t: Throwable) {
                Log.w(TAG, "TouchInteractionController registration failed, falling back to onMotionEvent", t)
            }
        }
    }

    override fun onInit(status: Int) {
        if (status == TextToSpeech.SUCCESS) {
            tts?.language = Locale.getDefault()
            isTtsReady = true
            Log.i(TAG, "Android TextToSpeech engine initialized successfully")
        } else {
            Log.e(TAG, "Android TextToSpeech initialization failed with status: $status")
        }
    }

    override fun onSpeakText(text: String, interrupt: Boolean) {
        val currentTts = tts ?: return

        if (interrupt) {
            currentTts.stop()
        }

        if (text.isNotBlank() && isTtsReady) {
            val queueMode = if (interrupt) TextToSpeech.QUEUE_FLUSH else TextToSpeech.QUEUE_ADD
            currentTts.speak(text, queueMode, null, "bit_sr_tts_${System.nanoTime()}")
        }
    }

    override fun onPerformAction(actionId: Int, targetNodeId: Long): Boolean {
        // Try finding the active accessibility focus first
        val focused = findFocus(AccessibilityNodeInfo.FOCUS_ACCESSIBILITY)
            ?: findFocus(AccessibilityNodeInfo.FOCUS_INPUT)

        if (focused != null) {
            val success = focused.performAction(actionId)
            focused.recycle()
            return success
        }

        // Fallback: execute on active window root
        val root = rootInActiveWindow
        if (root != null) {
            val success = root.performAction(actionId)
            root.recycle()
            return success
        }

        return false
    }

    /**
     * Fallback motion event receiver for devices below Android 13 with FLAG_SEND_MOTION_EVENTS.
     */
    override fun onMotionEvent(event: MotionEvent) {
        handleRawMotionEvent(event)
    }

    private fun handleRawMotionEvent(event: MotionEvent) {
        if (enginePtr == 0L) return

        val action = event.actionMasked
        val x = event.x
        val y = event.y
        val eventTime = event.eventTime

        // Zero-allocation raw register forwarding into native Rust
        NativeBridge.onRawTouch(enginePtr, action, x, y, eventTime)
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event == null || enginePtr == 0L) return

        val eventType = event.eventType
        val pkg = event.packageName?.toString() ?: ""
        val cls = event.className?.toString() ?: ""
        val text = event.text?.joinToString(" ") ?: ""
        val contentDesc = event.contentDescription?.toString() ?: ""

        val source = event.source
        var left = 0
        var top = 0
        var right = 0
        var bottom = 0
        var nodeId = 0L

        if (source != null) {
            source.getBoundsInScreen(tempRect)
            left = tempRect.left
            top = tempRect.top
            right = tempRect.right
            bottom = tempRect.bottom
            nodeId = source.hashCode().toLong()

            // If window hierarchy changed, harvest and update Rust cache
            if (eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED ||
                eventType == AccessibilityEvent.TYPE_WINDOWS_CHANGED) {
                harvestActiveWindowTree(source)
            }

            source.recycle()
        }

        NativeBridge.onAccessibilityEvent(
            enginePtr,
            eventType,
            pkg,
            cls,
            text,
            contentDesc,
            nodeId,
            left,
            top,
            right,
            bottom
        )
    }

    private fun harvestActiveWindowTree(root: AccessibilityNodeInfo) {
        // Collect window nodes into flat primitive buffers for zero-copy JNI transfer
        val nodeIds = ArrayList<Long>()
        val roles = ArrayList<Int>()
        val boundsLeft = ArrayList<Int>()
        val boundsTop = ArrayList<Int>()
        val boundsRight = ArrayList<Int>()
        val boundsBottom = ArrayList<Int>()
        val texts = ArrayList<String>()
        val states = ArrayList<Long>()

        fun traverse(node: AccessibilityNodeInfo) {
            node.getBoundsInScreen(tempRect)
            val label = (node.text ?: node.contentDescription ?: "").toString()
            
            nodeIds.add(node.hashCode().toLong())
            roles.add(0) // Mapped in Rust
            boundsLeft.add(tempRect.left)
            boundsTop.add(tempRect.top)
            boundsRight.add(tempRect.right)
            boundsBottom.add(tempRect.bottom)
            texts.add(label)
            
            var stateBits = 0L
            if (node.isFocused) stateBits = stateBits or 1L
            if (node.isAccessibilityFocused) stateBits = stateBits or 2L
            if (node.isChecked) stateBits = stateBits or 4L
            if (node.isSelected) stateBits = stateBits or 8L
            if (!node.isEnabled) stateBits = stateBits or 16L
            states.add(stateBits)

            for (i in 0 until node.childCount) {
                val child = node.getChild(i)
                if (child != null) {
                    traverse(child)
                    child.recycle()
                }
            }
        }

        traverse(root)

        if (nodeIds.isNotEmpty()) {
            NativeBridge.updateWindowTree(
                enginePtr,
                root.windowId,
                nodeIds.toLongArray(),
                roles.toIntArray(),
                boundsLeft.toIntArray(),
                boundsTop.toIntArray(),
                boundsRight.toIntArray(),
                boundsBottom.toIntArray(),
                texts.toTypedArray(),
                states.toLongArray()
            )
        }
    }

    override fun onInterrupt() {
        Log.i(TAG, "AccessibilityService interrupted - stopping speech")
        tts?.stop()
    }

    override fun onDestroy() {
        NativeBridge.hostCallback = null
        if (tts != null) {
            try {
                tts?.stop()
                tts?.shutdown()
            } catch (t: Throwable) {
                Log.w(TAG, "Error shutting down TextToSpeech", t)
            }
            tts = null
        }
        if (enginePtr != 0L) {
            NativeBridge.destroyEngine(enginePtr)
            enginePtr = 0L
        }
        super.onDestroy()
    }
}
