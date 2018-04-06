package net.mullvad.vpnapp

import java.util.concurrent.atomic.AtomicInteger

import kotlinx.coroutines.experimental.launch
import kotlinx.coroutines.experimental.CommonPool
import kotlinx.coroutines.experimental.CompletableDeferred
import kotlinx.coroutines.experimental.Deferred

import org.jetbrains.anko.AnkoLogger
import org.jetbrains.anko.debug
import org.jetbrains.anko.warn

import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener

import com.fasterxml.jackson.jr.ob.JSON

const val NORMAL_CLOSURE_STATUS = 1000

class JsonRpcWsClient(val uri: String, val httpClient: OkHttpClient = OkHttpClient()) : WebSocketListener(), AnkoLogger {
    data class QueuedRequest(val method: String, val promise: CompletableDeferred<Any>)

    private var connection: WebSocket? = null
    private var nextRequestId = AtomicInteger(1)
    private var requestQueue = ArrayList<QueuedRequest>()
    private var requestsInFlight = HashMap<Int, CompletableDeferred<Any>>()

    fun connect() {
        val request = Request.Builder().url(uri).build()

        httpClient.newWebSocket(request, this)
    }

    override fun onOpen(webSocket: WebSocket, response: Response) {
        val requestsToSend = dequeueRequests()

        connection = webSocket

        launch(CommonPool) {
            requestsToSend.iterator().forEach { request ->
                doCallMethod(request.method, request.promise, webSocket)
            }
        }
    }

    private fun dequeueRequests(): ArrayList<QueuedRequest> {
        synchronized(requestQueue) {
            val dequeuedRequests = requestQueue

            requestQueue = ArrayList()

            return dequeuedRequests
        }
    }

    override fun onMessage(webSocket: WebSocket, message: String) {
        val messageMap: Map<String, Any> = JSON.std.mapFrom(message)

        if (messageMap["jsonrpc"] == "2.0") {
            val id = messageMap["id"]
            val result = messageMap["result"]
            val response = requestsInFlight.remove(id)

            if (result != null) {
                response?.complete(result)
            } else {
                response?.completeExceptionally(Exception("Request failed"))
            }
        } else {
            warn("Received invalid message: $message")
        }
    }

    override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
        webSocket.close(NORMAL_CLOSURE_STATUS, null)
        connection = null
    }

    override fun onFailure(webSocket: WebSocket, error: Throwable, response: Response) {
        error("JSON RPC WebSocket client error: $error")
    }

    fun callMethod(method: String): Deferred<Any> {
        val promise: CompletableDeferred<Any> = CompletableDeferred()

        queueRequest(method, promise)

        return promise
    }

    private fun doCallMethod(method: String, promise: CompletableDeferred<Any>, connection: WebSocket) {
        val id = nextRequestId.addAndGet(1)

        connection.send("{\"jsonrpc\": \"2.0\", \"method\": \"$method\", \"id\": $id}")
        requestsInFlight.put(id, promise)
    }

    private fun queueRequest(method: String, promise: CompletableDeferred<Any>) {
        var connectionHandleAttempt: WebSocket? = null

        synchronized(requestQueue) {
            connectionHandleAttempt = connection

            if (connectionHandleAttempt == null) {
                requestQueue.add(QueuedRequest(method, promise))
            }
        }

        val connectionHandle = connectionHandleAttempt

        if (connectionHandle != null) {
            doCallMethod(method, promise, connectionHandle)
        }
    }

    fun getState(): Deferred<Any> {
        return callMethod("get_state")
    }
}
