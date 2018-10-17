// @flow

export type RetryAction = { action: 'stop' } | { action: 'retry', delay?: ?number };

export type RetryWatcher<T> = {
  onSuccess: (number, T) => void,
  onFailure: (number, any) => RetryAction,
};

export default class FetchRetry<T> {
  _fetch: () => Promise<T>;
  _watcher: ?RetryWatcher<T>;
  _fetchAttempt = 1;
  _scheduledRetry: ?TimeoutID;

  constructor(fetch: () => Promise<T>) {
    this._fetch = fetch;
  }

  fetch(onFailure: (number, any) => RetryAction): Promise<T> {
    return new Promise((resolve, reject) => {
      this.startFetching({
        onSuccess: (_, value) => resolve(value),
        onFailure: (attempt, error) => {
          const action = onFailure(attempt, error);
          if (action.action === 'stop') {
            reject(error);
          }
          return action;
        },
      });
    });
  }

  fetchOnce(onFailure: (number, any) => RetryAction, onUpdate: (T) => void): Promise<T> {
    return new Promise((resolve, reject) => {
      this.startFetching({
        onSuccess: (attempt, value) => {
          if (attempt === 1) {
            resolve(value);
          }
          onUpdate(value);
        },
        onFailure: (attempt, error) => {
          if (attempt === 1) {
            reject(error);
          }
          return onFailure(attempt, error);
        },
      });
    });
  }

  startFetching(watcher: RetryWatcher<T>) {
    this.cancel();

    this._watcher = watcher;
    this._fetchAttempt = 1;

    this._performFetch();
  }

  cancel() {
    this._watcher = null;

    if (this._scheduledRetry) {
      clearTimeout(this._scheduledRetry);
      this._scheduledRetry = null;
    }
  }

  async _performFetch() {
    const watcher = this._watcher;
    const fetchAttempt = this._fetchAttempt;

    if (!watcher) {
      return;
    }

    try {
      const value = await this._fetch();
      watcher.onSuccess(fetchAttempt, value);
    } catch (error) {
      const action = watcher.onFailure(fetchAttempt, error);

      // ensure a different fetch operation hasn't replaced the current one
      if (this._watcher === watcher) {
        if (action.action === 'retry') {
          this._fetchAttempt += 1;
          this._scheduledRetry = setTimeout(() => this._performFetch(), action.delay || 0);
        }
      }
    }
  }
}
