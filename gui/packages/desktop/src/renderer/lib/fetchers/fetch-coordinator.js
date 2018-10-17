// @flow

export default class FetchCoordinator<T> {
  _executingPromise: ?Promise<T>;
  _fetch: () => Promise<T>;

  constructor(fetch: () => Promise<T>) {
    this._fetch = fetch;
  }

  async fetch(): Promise<T> {
    // return the cached promise if there is an ongoing fetch
    if (this._executingPromise) {
      return this._executingPromise;
    }

    try {
      const fetchPromise = this._fetch();

      this._executingPromise = fetchPromise;

      const value = await fetchPromise;

      if (this._executingPromise === fetchPromise) {
        return value;
      } else {
        throw new Error('Cancelled');
      }
    } finally {
      this._executingPromise = null;
    }
  }

  cancel() {
    this._executingPromise = null;
  }
}
