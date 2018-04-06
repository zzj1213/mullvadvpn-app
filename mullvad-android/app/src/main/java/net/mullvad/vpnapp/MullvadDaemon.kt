package net.mullvad.vpnapp

import java.io.File
import java.io.FileOutputStream
import java.io.InputStream

import kotlin.io.*

import android.content.Context

private const val MULLVAD_DAEMON_EXE = "mullvad-daemon"
private const val MULLVAD_DAEMON_PATH = "/data/data/net.mullvad.vpnapp/files/mullvad-daemon"
private const val MULLVAD_RPC_ADDRESS_FILE = "/data/data/net.mullvad.vpnapp/files/.mullvad_rpc_address"

class MullvadDaemon {
    var rpcAddress: String? = null
        get() {
            if (field == null) {
                readRpcAddressFile()
            }

            return field
        }
        private set

    fun extract(context: Context) {
        if (!File(MULLVAD_DAEMON_PATH).canExecute()) {
            context
                .assets
                .open(MULLVAD_DAEMON_EXE)
                .copyTo(FileOutputStream(MULLVAD_DAEMON_PATH))

            Runtime.getRuntime().exec("/system/bin/chmod 750 $MULLVAD_DAEMON_PATH").waitFor()
        }
    }

    fun run(): Process {
        return ProcessBuilder(MULLVAD_DAEMON_PATH, "--disable-rpc-auth", "-vvv")
            .redirectErrorStream(true)
            .start()
    }

    private fun readRpcAddressFile() {
        var lines = File(MULLVAD_RPC_ADDRESS_FILE).readLines()

        rpcAddress = lines[0]
    }
}
