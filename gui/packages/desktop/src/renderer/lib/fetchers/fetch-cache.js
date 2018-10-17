// @flow

import FetchCoordinator from './fetch-coordinator';

export default class FetchCache<T> {
  _fetchCoordinator: FetchCoordinator<T>;
  _value: ?T;
  _expiration: Date | 'never' = new Date();
  _maxCacheAge: ?number;

  constructor(fetch: () => Promise<T>, maxCacheAge: ?number) {
    this._fetchCoordinator = new FetchCoordinator(fetch);
    this._maxCacheAge = maxCacheAge;
  }

  async fetch(): Promise<T> {
    if (this._value && !this._hasExpired()) {
      return this._value;
    } else {
      this._value = null;
    }

    const value = await this._fetchCoordinator.fetch();

    if (this._value) {
      // Cache has already been updated
      return this._value;
    } else {
      this.setValue(value);
      return value;
    }
  }

  invalidate() {
    this._expiration = new Date();
    this._value = null;
  }

  setValue(value: T) {
    this._value = value;

    if (this._maxCacheAge) {
      this._expiration = new Date(Date.now() + this._maxCacheAge);
    } else {
      this._expiration = 'never';
    }
  }

  _hasExpired(): boolean {
    return this._expiration !== 'never' && this._expiration >= new Date();
  }
}
