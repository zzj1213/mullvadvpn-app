// @flow

import { InvalidAccountError } from '../../errors';
import FetchCache from './fetch-cache';
import FetchRetry from './fetch-retry';

import type { AccountData, AccountToken, DaemonRpcProtocol } from '../daemon-rpc';
import type { RetryAction } from './fetch-retry';

const MAX_CACHE_AGE = 60 * 1000;

export default class AccountDataFetcher {
  _currentAccount: AccountToken;
  _cache: FetchCache<AccountData>;
  _retries: FetchRetry<AccountData>;
  _onUpdate: (AccountData) => void;

  constructor(rpc: DaemonRpcProtocol, onUpdate: (AccountData) => void) {
    this._cache = new FetchCache(() => {
      return rpc.getAccountData(this._currentAccount);
    }, MAX_CACHE_AGE);

    this._retries = new FetchRetry(() => this._cache.fetch());
    this._onUpdate = onUpdate;
  }

  invalidate() {
    this._retries.cancel();
    this._cache.invalidate();
  }

  async fetch(accountToken: AccountToken): Promise<AccountData> {
    if (accountToken != this._currentAccount) {
      this.invalidate();
      this._currentAccount = accountToken;
    }

    return this._retries.fetchOnce(this._onFetchError, this._onUpdate);
  }

  _onFetchError = (attempt: number, error: any): RetryAction => {
    if (error instanceof InvalidAccountError) {
      return { action: 'stop' };
    } else {
      return {
        action: 'retry',
        delay: Math.min(2048, 1 << (attempt + 2)) * 1000,
      };
    }
  };
}
