package net.mullvad.vpnapp

import kotlinx.coroutines.experimental.launch
import kotlinx.coroutines.experimental.withContext
import kotlinx.coroutines.experimental.CommonPool
import kotlinx.coroutines.experimental.android.UI

import android.app.Activity
import android.os.Bundle
import android.widget.TextView

import org.jetbrains.anko.button
import org.jetbrains.anko.scrollView
import org.jetbrains.anko.textResource
import org.jetbrains.anko.textView
import org.jetbrains.anko.toast
import org.jetbrains.anko.verticalLayout
import org.jetbrains.anko.sdk25.coroutines.onClick

import okhttp3.OkHttpClient
import okhttp3.Request

import net.mullvad.vpnapp.MullvadDaemon

class MainActivity : Activity() {
    private var logView: TextView? = null
    private lateinit var daemon: MullvadDaemon
    private var rpcClient: JsonRpcWsClient? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        daemon = MullvadDaemon()
        daemon.extract(this)

        verticalLayout {
            button {
                id = R.id.get_state
                textResource = R.string.get_state
                onClick {
                    getState()
                }
            }
            scrollView {
                textView {
                    id = R.id.log
                }
            }
        }

        logView = findViewById(R.id.log)

        launch(CommonPool) {
            val daemonProcess = daemon.run()
            val daemonLog = daemonProcess.inputStream.bufferedReader()

            daemonLog.forEachLine { line: String ->
                logLine(line)
            }

            logLine("\n\n FINISHED (" + daemonProcess.waitFor() + ")")
        }
    }

    private fun getState() {
        launch(CommonPool) {
            if (rpcClient == null) {
                val uri = daemon.rpcAddress

                if (uri != null) {
                    rpcClient = JsonRpcWsClient(uri)
                    rpcClient?.connect()
                }
            }

            val state = rpcClient?.getState()?.await()

            withContext(UI) {
                toast("State: $$state")
            }
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
