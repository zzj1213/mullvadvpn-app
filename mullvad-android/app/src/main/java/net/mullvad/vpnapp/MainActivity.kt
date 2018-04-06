package net.mullvad.vpnapp

import kotlinx.coroutines.experimental.launch
import kotlinx.coroutines.experimental.CommonPool
import kotlinx.coroutines.experimental.android.UI

import android.app.Activity
import android.os.Bundle
import android.widget.TextView

import org.jetbrains.anko.textView
import org.jetbrains.anko.scrollView

import net.mullvad.vpnapp.MullvadDaemon

class MainActivity : Activity() {
    private var logView: TextView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        var daemon = MullvadDaemon()
        daemon.extract(this)

        val layout = scrollView {
            textView {
                id = R.id.text
            }
        }

        setContentView(layout)

        logView = findViewById(R.id.text)

        launch(CommonPool) {
            val daemonProcess = daemon.run()
            val daemonLog = daemonProcess.inputStream.bufferedReader()

            daemonLog.forEachLine { line: String ->
                logLine(line)
            }

            logLine("\n\n FINISHED (" + daemonProcess.waitFor() + ")")
        }
    }

    private fun logLine(line: String) {
        launch(UI) {
            val logViewHandle = logView

            if (logViewHandle != null) {
                val previous: String = logViewHandle.text.toString()
                var lines: String = previous + "\n" + line

                logViewHandle.text = lines
            }
        }
    }
}
