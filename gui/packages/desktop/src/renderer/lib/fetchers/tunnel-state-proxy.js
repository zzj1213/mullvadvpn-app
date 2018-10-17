// @flow

import SubscriptionCache from './subscription-cache';
import { SubscriptionListener } from '../daemon-rpc';

import type { DaemonRpcProtocol, TunnelStateTransition } from '../daemon-rpc';

export default class TunnelStateProxy {
  _proxy: SubscriptionCache<TunnelStateTransition>;

  constructor(rpc: DaemonRpcProtocol, onUpdate: (TunnelStateTransition) => void) {
    this._proxy = new SubscriptionCache(
      () => rpc.getState(),
      (listener) => rpc.subscribeStateListener(new SubscriptionListener(listener, (_) => {})),
      onUpdate,
    );
    this._proxy.autoResubscribe(rpc);
  }

  fetch(): Promise<TunnelStateTransition> {
    return this._proxy.fetch();
  }
}
