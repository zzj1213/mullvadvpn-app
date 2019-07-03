import { app, nativeImage, NativeImage, Notification, shell } from 'electron';
import path from 'path';
import { sprintf } from 'sprintf-js';
import config from '../config.json';
import { TunnelState } from '../shared/daemon-rpc-types';
import { messages } from '../shared/gettext';

export default class NotificationController {
  private lastTunnelStateAnnouncement?: { body: string; notification: Notification };
  private reconnecting = false;
  private presentedNotifications: { [key: string]: boolean } = {};
  private pendingNotifications: Notification[] = [];
  private notificationTitle = process.platform === 'linux' ? app.getName() : '';
  private notificationIcon?: NativeImage;

  constructor() {
    if (process.platform === 'linux') {
      const basePath = path.resolve(path.join(__dirname, '../../assets/images'));
      this.notificationIcon = nativeImage.createFromPath(
        path.join(basePath, 'icon-notification.png'),
      );
    }
  }

  public notifyTunnelState(tunnelState: TunnelState) {
    switch (tunnelState.state) {
      case 'connecting':
        if (!this.reconnecting) {
          this.showTunnelStateNotification(messages.pgettext('notifications', 'Connecting'));
        }
        break;
      case 'connected':
        this.showTunnelStateNotification(messages.pgettext('notifications', 'Secured'));
        break;
      case 'disconnected':
        this.showTunnelStateNotification(messages.pgettext('notifications', 'Unsecured'));
        break;
      case 'blocked':
        switch (tunnelState.details.reason) {
          case 'set_firewall_policy_error':
            this.showTunnelStateNotification(
              messages.pgettext('notifications', 'Critical failure - Unsecured'),
            );
            break;
          default:
            this.showTunnelStateNotification(
              messages.pgettext('notifications', 'Blocked all connections'),
            );
            break;
        }
        break;
      case 'disconnecting':
        switch (tunnelState.details) {
          case 'nothing':
          case 'block':
            // no-op
            break;
          case 'reconnect':
            this.showTunnelStateNotification(messages.pgettext('notifications', 'Reconnecting'));
            this.reconnecting = true;
            return;
        }
        break;
    }

    this.reconnecting = false;
  }

  public notifyInconsistentVersion() {
    this.presentNotificationOnce('inconsistent-version', () => {
      const notification = new Notification({
        title: this.notificationTitle,
        body: messages.pgettext(
          'notifications',
          'Inconsistent internal version information, please restart the app',
        ),
        silent: true,
        icon: this.notificationIcon,
      });
      this.scheduleNotification(notification);
    });
  }

  public notifyUnsupportedVersion(upgradeVersion: string) {
    this.presentNotificationOnce('unsupported-version', () => {
      const notification = new Notification({
        title: this.notificationTitle,
        body: sprintf(
          // TRANSLATORS: The system notification displayed to the user when the running app becomes unsupported.
          // TRANSLATORS: Available placeholder:
          // TRANSLATORS: %(version) - the newest available version of the app
          messages.pgettext(
            'notifications',
            'You are running an unsupported app version. Please upgrade to %(version)s now to ensure your security',
          ),
          {
            version: upgradeVersion,
          },
        ),
        silent: true,
        icon: this.notificationIcon,
      });

      notification.on('click', () => {
        shell.openExternal(config.links.download);
      });

      this.scheduleNotification(notification);
    });
  }

  public cancelPendingNotifications() {
    for (const notification of this.pendingNotifications) {
      notification.close();
    }
  }

  public resetTunnelStateAnnouncements() {
    this.lastTunnelStateAnnouncement = undefined;
  }

  private showTunnelStateNotification(message: string) {
    const lastAnnouncement = this.lastTunnelStateAnnouncement;
    const sameAsLastNotification = lastAnnouncement && lastAnnouncement.body === message;

    if (sameAsLastNotification) {
      return;
    }

    const newNotification = new Notification({
      title: this.notificationTitle,
      body: message,
      silent: true,
      icon: this.notificationIcon,
    });

    if (lastAnnouncement) {
      lastAnnouncement.notification.close();
    }

    this.lastTunnelStateAnnouncement = {
      body: message,
      notification: newNotification,
    };

    this.scheduleNotification(newNotification);
  }

  private presentNotificationOnce(notificationName: string, presentNotification: () => void) {
    const presented = this.presentedNotifications;
    if (!presented[notificationName]) {
      presented[notificationName] = true;
      presentNotification();
    }
  }

  private scheduleNotification(notification: Notification) {
    this.addPendingNotification(notification);

    notification.show();

    setTimeout(() => notification.close(), 4000);
  }

  private addPendingNotification(notification: Notification) {
    notification.on('close', () => {
      this.removePendingNotification(notification);
    });

    this.pendingNotifications.push(notification);
  }

  private removePendingNotification(notification: Notification) {
    const index = this.pendingNotifications.indexOf(notification);
    if (index !== -1) {
      this.pendingNotifications.splice(index, 1);
    }
  }
}
