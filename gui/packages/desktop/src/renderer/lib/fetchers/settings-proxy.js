// @flow

import SubscriptionCache from './subscription-cache';
import { SubscriptionListener } from '../daemon-rpc';

import type { DaemonRpcProtocol, Settings } from '../daemon-rpc';

export default class SettingsProxy {
  _proxy: SubscriptionCache<Settings>;

  constructor(rpc: DaemonRpcProtocol, onUpdate: (Settings) => void) {
    this._proxy = new SubscriptionCache(
      () => rpc.getSettings(),
      (listener) => rpc.subscribeSettingsListener(new SubscriptionListener(listener, (_) => {})),
      onUpdate,
    );
    this._proxy.autoResubscribe(rpc);
  }

  fetch(): Promise<Settings> {
    return this._proxy.fetch();
  }
}
