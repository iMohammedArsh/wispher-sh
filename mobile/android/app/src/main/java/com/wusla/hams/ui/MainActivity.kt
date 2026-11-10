package com.wusla.hams.ui

import android.Manifest
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.mutableStateOf
import com.wusla.hams.ui.theme.HamsTheme

class MainActivity : ComponentActivity() {
    companion object {
        /** Which part of the app to open ("permission", "model", "settings"). */
        const val EXTRA_SECTION = "section"
    }

    private val section = mutableStateOf<String?>(null)
    private var onMicResult: ((Boolean) -> Unit)? = null
    private val micPermission = registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        onMicResult?.invoke(granted)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        section.value = intent?.getStringExtra(EXTRA_SECTION)
        setContent {
            HamsTheme {
                AppRoot(
                    section = section.value,
                    onSectionHandled = { section.value = null },
                    requestMic = { done ->
                        onMicResult = done
                        micPermission.launch(Manifest.permission.RECORD_AUDIO)
                    },
                )
            }
        }
    }

    override fun onNewIntent(intent: android.content.Intent) {
        super.onNewIntent(intent)
        section.value = intent.getStringExtra(EXTRA_SECTION)
    }
}
