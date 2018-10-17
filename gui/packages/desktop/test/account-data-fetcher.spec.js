// @flow

import AccountDataFetcher from '../src/renderer/lib/fetchers/account-data-fetcher';
import { InvalidAccountError } from '../src/renderer/errors';
import type { AccountData } from '../src/renderer/lib/daemon-rpc';

describe('AccountData fetcher', () => {
  const dummyAccountToken = '9876543210';
  const dummyAccountData: AccountData = {
    expiry: new Date('2038-01-01').toISOString(),
  };

  const mockRpc = () => {
    return spy.interface('daemonRpc', [
      'connect',
      'disconnect',
      'getRelayLocations',
      'setAccount',
      'updateRelaySettings',
      'setAllowLan',
      'setEnableIpv6',
      'setOpenVpnMssfix',
      'setAutoConnect',
      'connectTunnel',
      'disconnectTunnel',
      'getLocation',
      'getState',
      'getSettings',
      'subscribeStateListener',
      'subscribeSettingsListener',
      'addConnectionObserver',
      'removeConnectionObserver',
      'getAccountHistory',
      'removeAccountFromHistory',
      'getCurrentVersion',
      'getVersionInfo',
    ]);
  };

  let clock;

  beforeEach(() => {
    clock = sinon.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    clock.restore();
  });

  it('should notify when fetch succeeds on the first attempt', async () => {
    const rpc = mockRpc();
    const fetcher = new AccountDataFetcher(rpc, (_) => {});

    spy.on(rpc, 'getAccountData', (_) => Promise.resolve(dummyAccountData));

    const fetch = fetcher.fetch(dummyAccountToken);

    expect(fetch).to.eventually.be.fulfilled;
  });

  it('should notify when fetch fails on the first attempt', async () => {
    const rpc = mockRpc();
    const fetcher = new AccountDataFetcher(rpc, (_) => {});

    spy.on(rpc, 'getAccountData', (_) => Promise.reject(new Error('Fetch fail')));

    const fetch = fetcher.fetch(dummyAccountToken);

    expect(fetch).to.eventually.be.rejected;
  });

  it('should update when fetch succeeds on the first attempt', async () => {
    const update = new Promise((resolve) => {
      const rpc = mockRpc();
      const fetcher = new AccountDataFetcher(rpc, () => resolve());

      spy.on(rpc, 'getAccountData', (_) => Promise.resolve(dummyAccountData));

      fetcher.fetch(dummyAccountToken);
    });

    expect(update).to.eventually.be.fulfilled;
  });

  it('should update when fetch succeeds on the second attempt', async () => {
    const update = new Promise((resolve) => {
      const rpc = mockRpc();

      let firstAttempt = true;
      spy.on(rpc, 'getAccountData', (_) => {
        if (firstAttempt) {
          firstAttempt = false;
          setTimeout(() => clock.tick(9000), 0);
          return Promise.reject(new Error('First attempt fails'));
        } else {
          return Promise.resolve(dummyAccountData);
        }
      });

      const fetcher = new AccountDataFetcher(rpc, () => resolve());

      fetcher.fetch(dummyAccountToken);
    });

    expect(update).to.eventually.be.fulfilled;
  });

  it('should not retry if account is invalid', async () => {
    const rpc = mockRpc();

    const retry = new Promise((resolve, reject) => {
      let firstAttempt = true;
      spy.on(rpc, 'getAccountData', (_) => {
        if (firstAttempt) {
          firstAttempt = false;
          setTimeout(() => clock.tick(14000), 0);
          return Promise.reject(new InvalidAccountError());
        } else {
          reject();
          return Promise.resolve(dummyAccountData);
        }
      });

      setTimeout(resolve, 12000);
    });

    const update = new Promise((resolve) => {
      const fetcher = new AccountDataFetcher(rpc, () => resolve());

      fetcher.fetch(dummyAccountToken);
    });

    expect(update).to.eventually.be.fulfilled;
    expect(retry).to.eventually.be.fulfilled;
  });
});
