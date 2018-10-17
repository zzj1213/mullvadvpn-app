// @flow

import { ConnectionObserver } from '../daemon-rpc';
import type { DaemonRpcProtocol } from '../daemon-rpc';
import FetchCache from './fetch-cache';

export default class SubscriptionCache<T> {
  _isSubscribed = false;
  _fetchCache: FetchCache<T>;
  _subscribe: ((T) => void) => Promise<void>;
  _onUpdate: (T) => void;

  constructor(
    fetch: () => Promise<T>,
    subscribe: ((T) => void) => Promise<void>,
    onUpdate: (T) => void,
  ) {
    this._fetchCache = new FetchCache(fetch);
    this._subscribe = subscribe;
    this._onUpdate = onUpdate;
  }

  autoResubscribe(rpc: DaemonRpcProtocol) {
    rpc.addConnectionObserver(
      new ConnectionObserver(
        () => {
          this._doSubscribe();
        },
        () => {},
      ),
    );
  }

  async fetch(): Promise<T> {
    return this._fetchCache.fetch();
  }

  invalidate() {
    this._fetchCache.invalidate();
  }

  async _doSubscribe() {
    this._isSubscribed = false;
    await this._subscribe((value) => {
      this._fetchCache.setValue(value);
      this._onUpdate(value);
    });
    this._isSubscribed = true;

    this._onUpdate(await this.fetch());
  }
}
