// @flow

import log from 'electron-log';
import FetchRetry from './fetch-retry';

import type { DaemonRpcProtocol, Location } from '../daemon-rpc';
import type { RetryAction } from './fetch-retry';

export default class LocationFetcher {
  _retries: FetchRetry<Location>;

  constructor(rpc: DaemonRpcProtocol) {
    this._retries = new FetchRetry(() => {
      return rpc.getLocation();
    });
  }

  async fetch(): Promise<Location> {
    return this._retries.fetch(this._onFetchError);
  }

  _onFetchError = (attempt: number, error: any): RetryAction => {
    log.warn(`Failed to fetch location: ${error}`);
    return {
      action: 'retry',
      delay: Math.min(attempt * 500, 30000),
    };
  };
}
