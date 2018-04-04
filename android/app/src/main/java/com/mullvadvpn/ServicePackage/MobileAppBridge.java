package com.mullvadvpn.ServicePackage;

import android.app.Activity;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.net.Uri;
import android.support.annotation.Nullable;
import android.support.v4.content.FileProvider;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.Display;
import android.view.WindowManager;

import com.facebook.common.logging.FLog;
import com.facebook.react.bridge.Arguments;
import com.facebook.react.bridge.LifecycleEventListener;
import com.facebook.react.bridge.Promise;
import com.facebook.react.bridge.ReactApplicationContext;
import com.facebook.react.bridge.ReactContextBaseJavaModule;
import com.facebook.react.bridge.ReactMethod;
import com.facebook.react.bridge.WritableMap;
import com.facebook.react.modules.core.DeviceEventManagerModule;
import com.mullvadvpn.WireGuardVpnService;

import java.io.File;

import static android.content.Intent.FLAG_ACTIVITY_NEW_TASK;
import static android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION;

public class MobileAppBridge extends ReactContextBaseJavaModule implements LifecycleEventListener {
    private Intent serviceIntent;
    private final BroadcastReceiver receiver;
    private static final String TAG = "MobileAppBridge";

    public static final String MESSAGE = "message";
    public static final String ACTION_MESSAGE = "com.mullvad.rpc";
    public static final String ACTION_BACKEND_INFO = "com.mullvad.backend-info";
    public static final String BACKEND_INFO = "backend-info";
    public static final String BRIDGE_FILTER = "com.mullvad.MobileAppBridge";

    @Override
    public String getName() {
        return "MobileAppBridge";
    }

    public MobileAppBridge(ReactApplicationContext reactContext) {
        super(reactContext);
        receiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context context, Intent intent) {
                if (intent.getAction() != null && intent.getAction().equals(ACTION_MESSAGE)){
                    switch (intent.getAction()){
                        case ACTION_MESSAGE: sendToReact(intent.getAction(),intent.getStringExtra(MESSAGE));
                        case BACKEND_INFO: sendToReact(intent.getAction(),BACKEND_INFO);

                    }

                }
            }
        };
        reactContext.addLifecycleEventListener(this);
        serviceIntent = new Intent(reactContext, WireGuardVpnService.class);
    }

    private void sendToReact(String eventName,
                             @Nullable String msg) {
        WritableMap params = Arguments.createMap();
        params.putString(MESSAGE, msg);

        if (getReactApplicationContext() != null){
            getReactApplicationContext()
                    .getJSModule(DeviceEventManagerModule.RCTDeviceEventEmitter.class)
                    .emit(eventName, params);
        } else {
            Log.e(TAG, "Failed to send message to React");
        }
    }

    @ReactMethod
    public void sendRpc(String payload, Promise promise) {
        serviceIntent.putExtra(MESSAGE,payload);
        if (getReactApplicationContext() != null) {
            getReactApplicationContext().startService(serviceIntent);
            promise.resolve(true);
        } else {
            promise.reject("errorCode", "Could not get ReactApplicationContext");
        }
    }

    @ReactMethod
    public void openLogFile(){
        File file = new File(getReactApplicationContext().getFilesDir(), WireGuardVpnService.LOG_FILE_NAME);

        Uri contentUri = FileProvider.getUriForFile(getReactApplicationContext(), "com.mullvad.FileProvider", file);

        Intent shareIntent = new Intent(Intent.ACTION_SEND);
        shareIntent.putExtra(Intent.EXTRA_STREAM, contentUri);
        shareIntent.setType("text/plain");
        Intent chooseIntent = Intent.createChooser(shareIntent, "View log file");
        chooseIntent.setFlags(FLAG_ACTIVITY_NEW_TASK);
        chooseIntent.setFlags(FLAG_GRANT_READ_URI_PERMISSION);
        getReactApplicationContext().startActivity(chooseIntent);
    }

    @ReactMethod
    public void startBackend(Promise promise) {
        if (getReactApplicationContext() != null) {
            Intent intent = WireGuardVpnService.prepare(getReactApplicationContext());
            if (intent != null) {
                getReactApplicationContext().startActivityForResult(intent, 0,null);
            } else {
                getReactApplicationContext().startService(serviceIntent);
            }
            promise.resolve(true);
        } else {
            promise.reject("errorCode", "Could not get ReactApplicationContext");
        }
    }

    @ReactMethod
    public void getMenuBarHeight(Promise promise) {
        try {
            Display d = ((WindowManager) getReactApplicationContext().getSystemService(Context.WINDOW_SERVICE)).getDefaultDisplay();

            DisplayMetrics realDisplayMetrics = new DisplayMetrics();
            d.getRealMetrics(realDisplayMetrics);

            int realHeight = realDisplayMetrics.heightPixels;
            int realWidth = realDisplayMetrics.widthPixels;

            DisplayMetrics displayMetrics = new DisplayMetrics();
            d.getMetrics(displayMetrics);

            int displayHeight = displayMetrics.heightPixels;
            int displayWidth = displayMetrics.widthPixels;
            Log.d(TAG,"displayHeight:" + displayHeight);
            Log.d(TAG,"realHeight:" + realHeight);
            Log.d(TAG,"displayMetrics.density:" + displayMetrics.density);


            promise.resolve(Math.max(0, (realHeight - displayHeight) / displayMetrics.density));
        } catch (Exception e) {
            promise.reject("errorCode", "Could not get Screen dimensions");
        }
    }

    @ReactMethod
    public void openItem(String path) {
        //TODO: probably make a small activity to view text instead of intent.
        Intent intent = new Intent(Intent.ACTION_VIEW);
        Uri uri = Uri.parse(path);
        intent.setDataAndType(uri, "text/plain");
        getReactApplicationContext().startActivity(intent);
    }

    @Override
    public void onHostResume() {
        Log.d(TAG,"onHostResume");
        final Activity activity = getCurrentActivity();

        if (activity == null) {
            Log.e(TAG, "no activity to register receiver");
            return;
        }
        activity.registerReceiver(receiver, new IntentFilter(BRIDGE_FILTER));
    }
    @Override
    public void onHostPause() {
        Log.d(TAG,"onHostPause");
        final Activity activity = getCurrentActivity();
        if (activity == null) return;
        try
        {
            activity.unregisterReceiver(receiver);
        }
        catch (java.lang.IllegalArgumentException e) {
            FLog.e(TAG, "receiver already unregistered", e);
        }
    }

    @Override
    public void onHostDestroy() {
        Log.d(TAG,"onHostDestroy");
    }
}