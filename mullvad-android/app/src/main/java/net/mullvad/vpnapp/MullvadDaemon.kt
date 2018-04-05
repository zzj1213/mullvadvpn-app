package net.mullvad.vpnapp

import java.io.File
import java.io.FileOutputStream
import java.io.InputStream

import kotlin.io.*

import android.content.Context

private const val MULLVAD_DAEMON_EXE = "mullvad-daemon"
private const val MULLVAD_DAEMON_PATH = "/data/data/net.mullvad.vpnapp/files/mullvad-daemon"

class MullvadDaemon {
    fun extract(context: Context) {
        if (!File(MULLVAD_DAEMON_PATH).canExecute()) {
            context
                .assets
                .open(MULLVAD_DAEMON_EXE)
                .copyTo(FileOutputStream(MULLVAD_DAEMON_PATH))

            Runtime.getRuntime().exec("/system/bin/chmod 750 $MULLVAD_DAEMON_PATH").waitFor()
        }
    }
}
