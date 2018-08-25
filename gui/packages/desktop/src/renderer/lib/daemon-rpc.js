// @flow

import JsonRpcTransport, {
  RemoteError as JsonRpcRemoteError,
  TimeOutError as JsonRpcTimeOutError,
} from './jsonrpc-transport';
import { CommunicationError, InvalidAccountError, NoDaemonError } from '../errors';

import {
  object,
  maybe,
  string,
  number,
  boolean,
  enumeration,
  arrayOf,
  oneOf,
} from 'validated/schema';
import { validate } from 'validated/object';

import type { Node as SchemaNode } from 'validated/schema';

export type AccountData = { expiry: string };
export type AccountToken = string;
export type Ip = string;
export type Location = {
  ip: Ip,
  country: string,
  city: ?string,
  latitude: number,
  longitude: number,
  mullvad_exit_ip: boolean,
};
const LocationSchema = object({
  ip: string,
  country: string,
  city: maybe(string),
  latitude: number,
  longitude: number,
  mullvad_exit_ip: boolean,
});

export type SecurityState = 'secured' | 'unsecured';
export type BackendState = {
  state: SecurityState,
  target_state: SecurityState,
};

export type RelayProtocol = 'tcp' | 'udp';
export type RelayLocation = {| city: [string, string] |} | {| country: string |};

type OpenVpnConstraints = {
  port: 'any' | { only: number },
  protocol: 'any' | { only: RelayProtocol },
};

type TunnelConstraints<TOpenVpnConstraints> = {
  openvpn: TOpenVpnConstraints,
};

type RelaySettingsNormal<TTunnelConstraints> = {
  location:
    | 'any'
    | {
        only: RelayLocation,
      },
  tunnel:
    | 'any'
    | {
        only: TTunnelConstraints,
      },
};

// types describing the structure of RelaySettings
export type RelaySettingsCustom = {
  host: string,
  tunnel: {
    openvpn: {
      port: number,
      protocol: RelayProtocol,
    },
  },
};
export type RelaySettings =
  | {|
      normal: RelaySettingsNormal<TunnelConstraints<OpenVpnConstraints>>,
    |}
  | {|
      custom_tunnel_endpoint: RelaySettingsCustom,
    |};

// types describing the partial update of RelaySettings
export type RelaySettingsNormalUpdate = $Shape<
  RelaySettingsNormal<TunnelConstraints<$Shape<OpenVpnConstraints>>>,
>;
export type RelaySettingsUpdate =
  | {|
      normal: RelaySettingsNormalUpdate,
    |}
  | {|
      custom_tunnel_endpoint: RelaySettingsCustom,
    |};

const constraint = <T>(constraintValue: SchemaNode<T>) => {
  return oneOf(
    string, // any
    object({
      only: constraintValue,
    }),
  );
};

const RelaySettingsSchema = oneOf(
  object({
    normal: object({
      location: constraint(
        oneOf(
          object({
            city: arrayOf(string),
          }),
          object({
            country: string,
          }),
        ),
      ),
      tunnel: constraint(
        object({
          openvpn: object({
            port: constraint(number),
            protocol: constraint(enumeration('udp', 'tcp')),
          }),
        }),
      ),
    }),
  }),
  object({
    custom_tunnel_endpoint: object({
      host: string,
      tunnel: object({
        openvpn: object({
          port: number,
          protocol: enumeration('udp', 'tcp'),
        }),
      }),
    }),
  }),
);

export type RelayList = {
  countries: Array<RelayListCountry>,
};

export type RelayListCountry = {
  name: string,
  code: string,
  cities: Array<RelayListCity>,
};

export type RelayListCity = {
  name: string,
  code: string,
  latitude: number,
  longitude: number,
  has_active_relays: boolean,
};

const RelayListSchema = object({
  countries: arrayOf(
    object({
      name: string,
      code: string,
      cities: arrayOf(
        object({
          name: string,
          code: string,
          latitude: number,
          longitude: number,
          has_active_relays: boolean,
        }),
      ),
    }),
  ),
});

export type TunnelOptions = {
  openvpn: {
    enableIpv6: boolean,
  },
};

const TunnelOptionsSchema = object({
  openvpn: object({
    enable_ipv6: boolean,
    mssfix: maybe(number),
  }),
});

const AccountDataSchema = object({
  expiry: string,
});

const allSecurityStates: Array<SecurityState> = ['secured', 'unsecured'];
const BackendStateSchema = object({
  state: enumeration(...allSecurityStates),
  target_state: enumeration(...allSecurityStates),
});

export type AppVersionInfo = {
  currentIsSupported: boolean,
  latest: {
    latestStable: string,
    latest: string,
  },
};

const AppVersionInfoSchema = object({
  current_is_supported: boolean,
  latest: object({
    latest_stable: string,
    latest: string,
  }),
});

export interface DaemonRpcProtocol {
  connect(string): void;
  disconnect(): void;
  getAccountData(AccountToken): Promise<AccountData>;
  getRelayLocations(): Promise<RelayList>;
  getAccount(): Promise<?AccountToken>;
  setAccount(accountToken: ?AccountToken): Promise<void>;
  updateRelaySettings(RelaySettingsUpdate): Promise<void>;
  getRelaySettings(): Promise<RelaySettings>;
  setAllowLan(boolean): Promise<void>;
  getAllowLan(): Promise<boolean>;
  setOpenVpnEnableIpv6(boolean): Promise<void>;
  getTunnelOptions(): Promise<TunnelOptions>;
  setAutoConnect(boolean): Promise<void>;
  getAutoConnect(): Promise<boolean>;
  connectTunnel(): Promise<void>;
  disconnectTunnel(): Promise<void>;
  getLocation(): Promise<Location>;
  getState(): Promise<BackendState>;
  subscribeStateListener((state: ?BackendState, error: ?Error) => void): Promise<void>;
  addOpenConnectionObserver(() => void): ConnectionObserver;
  addCloseConnectionObserver((error: ?Error) => void): ConnectionObserver;
  authenticate(sharedSecret: string): Promise<void>;
  getAccountHistory(): Promise<Array<AccountToken>>;
  removeAccountFromHistory(accountToken: AccountToken): Promise<void>;
  getCurrentVersion(): Promise<string>;
  getVersionInfo(): Promise<AppVersionInfo>;
}

export class ResponseParseError extends Error {
  _validationError: ?Error;

  constructor(message: string, validationError: ?Error) {
    super(message);
    this._validationError = validationError;
  }

  get validationError(): ?Error {
    return this._validationError;
  }
}

export type ConnectionObserver = {
  unsubscribe: () => void,
};

export class DaemonRpc implements DaemonRpcProtocol {
  _transport = new JsonRpcTransport();

  async authenticate(sharedSecret: string): Promise<void> {
    await this._transport.send('auth', sharedSecret);
  }

  connect(connectionString: string) {
    this._transport.connect(connectionString);
  }

  disconnect() {
    this._transport.disconnect();
  }

  addOpenConnectionObserver(handler: () => void): ConnectionObserver {
    this._transport.on('open', handler);
    return {
      unsubscribe: () => {
        this._transport.off('open', handler);
      },
    };
  }

  addCloseConnectionObserver(handler: (error: ?Error) => void): ConnectionObserver {
    this._transport.on('close', handler);
    return {
      unsubscribe: () => {
        this._transport.off('close', handler);
      },
    };
  }

  async _call<T>(method: string, schema: mixed, data: mixed, timeout?: number): Promise<T> {
    let response;
    try {
      response = await this._transport.send(method, data, timeout);
    } catch (error) {
      if (error instanceof JsonRpcRemoteError) {
        switch (error.code) {
          case -10000: // Internal error
            throw new Error('Unexpected internal error');
          case -10100: // Unknown API server error
            throw new Error('Unexpected API server error');
          case -10101: // Communication with API server error
            throw new CommunicationError();
          case -10200: // Account doesn't exist
            throw new InvalidAccountError();
          default:
            throw error;
        }
      } else if (error instanceof JsonRpcTimeOutError) {
        throw new NoDaemonError();
      } else {
        throw error;
      }
    }

    try {
      if (typeof schema === 'function') {
        if (!schema(response)) {
          throw new Error('Validation failed');
        } else {
          return response;
        }
      } else {
        return validate(schema, response);
      }
    } catch (error) {
      throw new ResponseParseError(`Invalid response from ${method}`, error);
    }
  }

  async getAccountData(accountToken: AccountToken): Promise<AccountData> {
    // send the IPC with 30s timeout since the backend will wait
    // for a HTTP request before replying
    return this._call('get_account_data', AccountDataSchema, accountToken, 30000);
  }

  async getRelayLocations(): Promise<RelayList> {
    return this._call('get_relay_locations', RelayListSchema);
  }

  async getAccount(): Promise<?AccountToken> {
    return this._call(
      'get_account',
      (response) => response === null || typeof response === 'string',
    );
  }

  async setAccount(accountToken: ?AccountToken): Promise<void> {
    await this._transport.send('set_account', accountToken);
  }

  async updateRelaySettings(relaySettings: RelaySettingsUpdate): Promise<void> {
    await this._transport.send('update_relay_settings', [relaySettings]);
  }

  async getRelaySettings(): Promise<RelaySettings> {
    const validatedObject = await this._call('get_relay_settings', RelaySettingsSchema);

    /* $FlowFixMe:
      There is no way to express constraints with string literals, i.e:

      RelaySettingsSchema constraint:
        oneOf(string, object)

      RelaySettings constraint:
        'any' | object

      These two are incompatible so we simply enforce the type for now.
    */
    return ((validatedObject: any): RelaySettings);
  }

  async setAllowLan(allowLan: boolean): Promise<void> {
    await this._transport.send('set_allow_lan', [allowLan]);
  }

  async getAllowLan(): Promise<boolean> {
    return this._call('get_allow_lan', boolean);
  }

  async setOpenVpnEnableIpv6(enableIpv6: boolean): Promise<void> {
    await this._transport.send('set_openvpn_enable_ipv6', [enableIpv6]);
  }

  async getTunnelOptions(): Promise<TunnelOptions> {
    const validatedObject = await this._call('get_tunnel_options', TunnelOptionsSchema);

    return {
      openvpn: {
        enableIpv6: validatedObject.openvpn.enable_ipv6,
      },
    };
  }

  async setAutoConnect(autoConnect: boolean): Promise<void> {
    await this._transport.send('set_auto_connect', [autoConnect]);
  }

  async getAutoConnect(): Promise<boolean> {
    return this._call('get_auto_connect', boolean);
  }

  async connectTunnel(): Promise<void> {
    await this._transport.send('connect');
  }

  async disconnectTunnel(): Promise<void> {
    await this._transport.send('disconnect');
  }

  async getLocation(): Promise<Location> {
    // send the IPC with 30s timeout since the backend will wait
    // for a HTTP request before replying
    return this._call('get_current_location', LocationSchema, [], 30000);
  }

  async getState(): Promise<BackendState> {
    return this._call('get_state', BackendStateSchema);
  }

  subscribeStateListener(listener: (state: ?BackendState, error: ?Error) => void): Promise<void> {
    return this._transport.subscribe('new_state', (payload) => {
      try {
        const newState = validate(BackendStateSchema, payload);
        listener(newState, null);
      } catch (error) {
        listener(null, new ResponseParseError('Invalid payload from new_state', error));
      }
    });
  }

  async getAccountHistory(): Promise<Array<AccountToken>> {
    return this._call('get_account_history', arrayOf(string));
  }

  async removeAccountFromHistory(accountToken: AccountToken): Promise<void> {
    await this._transport.send('remove_account_from_history', accountToken);
  }

  async getCurrentVersion(): Promise<string> {
    return this._call('get_current_version', string);
  }

  async getVersionInfo(): Promise<AppVersionInfo> {
    const versionInfo = await this._call('get_version_info', AppVersionInfoSchema);
    return {
      currentIsSupported: versionInfo.current_is_supported,
      latest: {
        latestStable: versionInfo.latest.latest_stable,
        latest: versionInfo.latest.latest,
      },
    };
  }
}
