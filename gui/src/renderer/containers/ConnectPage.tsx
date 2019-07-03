import { push } from 'connected-react-router';
import { shell } from 'electron';
import log from 'electron-log';
import { connect } from 'react-redux';
import { bindActionCreators } from 'redux';
import { sprintf } from 'sprintf-js';
import {
  countries,
  messages,
  relayLocations as relayLocationsLocalization,
} from '../../shared/gettext';
import Connect from '../components/Connect';
import AccountExpiry from '../lib/account-expiry';
import { IRelayLocationRedux, RelaySettingsRedux } from '../redux/settings/reducers';
import { IReduxState, ReduxDispatch } from '../redux/store';
import { ISharedRouteProps } from '../routes';

function getRelayName(
  relaySettings: RelaySettingsRedux,
  relayLocations: IRelayLocationRedux[],
): string {
  if ('normal' in relaySettings) {
    const location = relaySettings.normal.location;

    if (location === 'any') {
      return 'Automatic';
    } else if ('country' in location) {
      const country = relayLocations.find(({ code }) => code === location.country);
      if (country) {
        return countries.gettext(country.name);
      }
    } else if ('city' in location) {
      const [countryCode, cityCode] = location.city;
      const country = relayLocations.find(({ code }) => code === countryCode);
      if (country) {
        const city = country.cities.find(({ code }) => code === cityCode);
        if (city) {
          return relayLocationsLocalization.gettext(city.name);
        }
      }
    } else if ('hostname' in location) {
      const [countryCode, cityCode, hostname] = location.hostname;
      const country = relayLocations.find(({ code }) => code === countryCode);
      if (country) {
        const city = country.cities.find(({ code }) => code === cityCode);
        if (city) {
          return sprintf(
            // TRANSLATORS: The selected location label displayed on the main view, when a user selected a specific host to connect to.
            // TRANSLATORS: Example: Malmö (se-mma-001)
            // TRANSLATORS: Available placeholders:
            // TRANSLATORS: %(city)s - a city name
            // TRANSLATORS: %(hostname)s - a hostname
            messages.pgettext('connect-container', '%(city)s (%(hostname)s)'),
            {
              city: city.name,
              hostname,
            },
          );
        }
      }
    }

    return 'Unknown';
  } else if (relaySettings.customTunnelEndpoint) {
    return 'Custom';
  } else {
    throw new Error('Unsupported relay settings.');
  }
}

const mapStateToProps = (state: IReduxState, props: ISharedRouteProps) => {
  return {
    accountExpiry: state.account.expiry
      ? new AccountExpiry(state.account.expiry, props.locale)
      : undefined,
    selectedRelayName: getRelayName(state.settings.relaySettings, state.settings.relayLocations),
    connection: state.connection,
    version: state.version,
    blockWhenDisconnected: state.settings.blockWhenDisconnected,
  };
};

const mapDispatchToProps = (dispatch: ReduxDispatch, props: ISharedRouteProps) => {
  const history = bindActionCreators({ push }, dispatch);

  return {
    onSettings: () => {
      history.push('/settings');
    },
    onSelectLocation: () => {
      history.push('/select-location');
    },
    onConnect: async () => {
      try {
        await props.app.connectTunnel();
      } catch (error) {
        log.error(`Failed to connect the tunnel: ${error.message}`);
      }
    },
    onDisconnect: async () => {
      try {
        await props.app.disconnectTunnel();
      } catch (error) {
        log.error(`Failed to disconnect the tunnel: ${error.message}`);
      }
    },
    onExternalLink: (url: string) => shell.openExternal(url),
  };
};

export default connect(
  mapStateToProps,
  mapDispatchToProps,
)(Connect);
