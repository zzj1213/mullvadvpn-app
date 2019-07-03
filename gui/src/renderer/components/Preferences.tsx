import * as React from 'react';
import { Component, View } from 'reactxp';
import { messages } from '../../shared/gettext';
import * as Cell from './Cell';
import { Container, Layout } from './Layout';
import {
  BackBarItem,
  NavigationBar,
  NavigationContainer,
  NavigationScrollbars,
  TitleBarItem,
} from './NavigationBar';
import styles from './PreferencesStyles';
import SettingsHeader, { HeaderTitle } from './SettingsHeader';

export interface IPreferencesProps {
  autoStart: boolean;
  autoConnect: boolean;
  allowLan: boolean;
  enableSystemNotifications: boolean;
  monochromaticIcon: boolean;
  startMinimized: boolean;
  enableMonochromaticIconToggle: boolean;
  enableStartMinimizedToggle: boolean;
  setAutoStart: (autoStart: boolean) => void;
  setEnableSystemNotifications: (flag: boolean) => void;
  setAutoConnect: (autoConnect: boolean) => void;
  setAllowLan: (allowLan: boolean) => void;
  setStartMinimized: (startMinimized: boolean) => void;
  setMonochromaticIcon: (monochromaticIcon: boolean) => void;
  onClose: () => void;
}

export default class Preferences extends Component<IPreferencesProps> {
  public render() {
    return (
      <Layout>
        <Container>
          <View style={styles.preferences}>
            <NavigationContainer>
              <NavigationBar>
                <BackBarItem action={this.props.onClose}>
                  {// TRANSLATORS: Back button in navigation bar
                  messages.pgettext('preferences-nav', 'Settings')}
                </BackBarItem>
                <TitleBarItem>
                  {// TRANSLATORS: Title label in navigation bar
                  messages.pgettext('preferences-nav', 'Preferences')}
                </TitleBarItem>
              </NavigationBar>

              <View style={styles.preferences__container}>
                <NavigationScrollbars>
                  <SettingsHeader>
                    <HeaderTitle>
                      {messages.pgettext('preferences-view', 'Preferences')}
                    </HeaderTitle>
                  </SettingsHeader>

                  <View style={styles.preferences__content}>
                    <Cell.Container>
                      <Cell.Label>
                        {messages.pgettext('preferences-view', 'Launch app on start-up')}
                      </Cell.Label>
                      <Cell.Switch isOn={this.props.autoStart} onChange={this.onChangeAutoStart} />
                    </Cell.Container>
                    <View style={styles.preferences__separator} />

                    <Cell.Container>
                      <Cell.Label>
                        {messages.pgettext('preferences-view', 'Auto-connect')}
                      </Cell.Label>
                      <Cell.Switch
                        isOn={this.props.autoConnect}
                        onChange={this.props.setAutoConnect}
                      />
                    </Cell.Container>
                    <Cell.Footer>
                      {messages.pgettext(
                        'preferences-view',
                        'Automatically connect to a server when the app launches.',
                      )}
                    </Cell.Footer>

                    <Cell.Container>
                      <Cell.Label>
                        {messages.pgettext('preferences-view', 'Local network sharing')}
                      </Cell.Label>
                      <Cell.Switch isOn={this.props.allowLan} onChange={this.props.setAllowLan} />
                    </Cell.Container>
                    <Cell.Footer>
                      {messages.pgettext(
                        'preferences-view',
                        'Allows access to other devices on the same network for sharing, printing etc.',
                      )}
                    </Cell.Footer>

                    <Cell.Container>
                      <Cell.Label>
                        {messages.pgettext('preferences-view', 'Notifications')}
                      </Cell.Label>
                      <Cell.Switch
                        isOn={this.props.enableSystemNotifications}
                        onChange={this.props.setEnableSystemNotifications}
                      />
                    </Cell.Container>
                    <Cell.Footer>
                      {messages.pgettext(
                        'preferences-view',
                        'Enable or disable system notifications. The critical notifications will always be displayed.',
                      )}
                    </Cell.Footer>

                    <MonochromaticIconToggle
                      enable={this.props.enableMonochromaticIconToggle}
                      monochromaticIcon={this.props.monochromaticIcon}
                      onChange={this.props.setMonochromaticIcon}
                    />

                    <StartMinimizedToggle
                      enable={this.props.enableStartMinimizedToggle}
                      startMinimized={this.props.startMinimized}
                      onChange={this.props.setStartMinimized}
                    />
                  </View>
                </NavigationScrollbars>
              </View>
            </NavigationContainer>
          </View>
        </Container>
      </Layout>
    );
  }

  private onChangeAutoStart = (autoStart: boolean) => {
    this.props.setAutoStart(autoStart);
  };
}

interface IMonochromaticIconProps {
  enable: boolean;
  monochromaticIcon: boolean;
  onChange: (value: boolean) => void;
}

class MonochromaticIconToggle extends Component<IMonochromaticIconProps> {
  public render() {
    if (this.props.enable) {
      return (
        <View>
          <Cell.Container>
            <Cell.Label>
              {messages.pgettext('preferences-view', 'Monochromatic tray icon')}
            </Cell.Label>
            <Cell.Switch isOn={this.props.monochromaticIcon} onChange={this.props.onChange} />
          </Cell.Container>
          <Cell.Footer>
            {messages.pgettext(
              'preferences-view',
              'Use a monochromatic tray icon instead of a colored one.',
            )}
          </Cell.Footer>
        </View>
      );
    } else {
      return null;
    }
  }
}

interface IStartMinimizedProps {
  enable: boolean;
  startMinimized: boolean;
  onChange: (value: boolean) => void;
}

class StartMinimizedToggle extends Component<IStartMinimizedProps> {
  public render() {
    if (this.props.enable) {
      return (
        <View>
          <Cell.Container>
            <Cell.Label>{messages.pgettext('preferences-view', 'Start minimized')}</Cell.Label>
            <Cell.Switch isOn={this.props.startMinimized} onChange={this.props.onChange} />
          </Cell.Container>
          <Cell.Footer>
            {messages.pgettext('preferences-view', 'Show only the tray icon when the app starts.')}
          </Cell.Footer>
        </View>
      );
    } else {
      return null;
    }
  }
}
