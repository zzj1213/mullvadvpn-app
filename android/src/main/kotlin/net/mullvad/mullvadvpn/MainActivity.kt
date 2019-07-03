package net.mullvad.mullvadvpn

import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.Job

import android.app.Activity
import android.content.ComponentName
import android.content.Intent
import android.content.ServiceConnection
import android.net.VpnService
import android.os.Bundle
import android.os.IBinder
import android.support.v4.app.FragmentActivity

import net.mullvad.mullvadvpn.dataproxy.AccountCache
import net.mullvad.mullvadvpn.dataproxy.ConnectionProxy
import net.mullvad.mullvadvpn.dataproxy.LocationInfoCache
import net.mullvad.mullvadvpn.dataproxy.MullvadProblemReport
import net.mullvad.mullvadvpn.dataproxy.RelayListListener
import net.mullvad.mullvadvpn.dataproxy.SettingsListener
import net.mullvad.mullvadvpn.model.RelaySettings
import net.mullvad.mullvadvpn.model.Settings
import net.mullvad.mullvadvpn.relaylist.RelayItem
import net.mullvad.mullvadvpn.relaylist.RelayList

class MainActivity : FragmentActivity() {
    private var vpnPermission: CompletableDeferred<Boolean>? = null

    var daemon = CompletableDeferred<MullvadDaemon>()
        private set

    val connectionProxy = ConnectionProxy(this)
    val locationInfoCache = LocationInfoCache(daemon)
    val problemReport = MullvadProblemReport()
    var settingsListener = SettingsListener(this)
    var relayListListener = RelayListListener(this)
    val accountCache = AccountCache(settingsListener, daemon)

    private var waitForDaemonJob: Job? = null

    private val serviceConnection = object : ServiceConnection {
        override fun onServiceConnected(className: ComponentName, binder: IBinder) {
            val localBinder = binder as MullvadVpnService.LocalBinder

            waitForDaemonJob = GlobalScope.launch(Dispatchers.Default) {
                daemon.complete(localBinder.daemon.await())
            }
        }

        override fun onServiceDisconnected(className: ComponentName) {
            daemon.cancel()
            daemon = CompletableDeferred<MullvadDaemon>()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.main)

        if (savedInstanceState == null) {
            addInitialFragment()
        }
    }

    override fun onStart() {
        super.onStart()

        val intent = Intent(this, MullvadVpnService::class.java)

        startService(intent)
        bindService(intent, serviceConnection, 0)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, resultData: Intent?) {
        if (resultCode == Activity.RESULT_OK) {
            vpnPermission?.complete(true)
        } else {
            vpnPermission?.complete(false)
        }
    }

    override fun onStop() {
        unbindService(serviceConnection)

        super.onStop()
    }

    override fun onDestroy() {
        accountCache.onDestroy()
        relayListListener.onDestroy()
        settingsListener.onDestroy()

        waitForDaemonJob?.cancel()
        daemon.cancel()

        super.onDestroy()
    }

    fun openSettings() {
        supportFragmentManager?.beginTransaction()?.apply {
            setCustomAnimations(
                R.anim.fragment_enter_from_bottom,
                R.anim.do_nothing,
                R.anim.do_nothing,
                R.anim.fragment_exit_to_bottom
            )
            replace(R.id.main_fragment, SettingsFragment())
            addToBackStack(null)
            commit()
        }
    }

    fun requestVpnPermission(): Deferred<Boolean> {
        val intent = VpnService.prepare(this)
        val request = CompletableDeferred<Boolean>()

        vpnPermission = request

        if (intent != null) {
            startActivityForResult(intent, 0)
        } else {
            request.complete(true)
        }

        return request
    }

    private fun addInitialFragment() {
        supportFragmentManager?.beginTransaction()?.apply {
            add(R.id.main_fragment, LaunchFragment())
            commit()
        }
    }

    private fun fetchSettings() = GlobalScope.async(Dispatchers.Default) {
        daemon.await().getSettings()
    }
}
