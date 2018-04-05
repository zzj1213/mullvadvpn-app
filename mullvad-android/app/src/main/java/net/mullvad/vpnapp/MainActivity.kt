package net.mullvad.vpnapp

import android.app.Activity
import android.os.Bundle

import net.mullvad.vpnapp.MullvadDaemon

class MainActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        var daemon = MullvadDaemon()
        daemon.extract(this)

        setContentView(R.layout.main_activity)
    }
}
