import moment from 'moment';
import { sprintf } from 'sprintf-js';
import { pgettext } from '../../shared/gettext';

export default class AccountExpiry {
  private expiry: moment.Moment;

  constructor(isoString: string, locale: string) {
    this.expiry = moment(isoString).locale(locale);
  }

  public hasExpired(): boolean {
    return this.willHaveExpiredAt(new Date());
  }

  public willHaveExpiredAt(date: Date): boolean {
    return this.expiry.isSameOrBefore(date);
  }

  public formattedDate(): string {
    return this.expiry.format('L LTS');
  }

  public remainingTime(): string {
    const duration = this.expiry.fromNow(true);

    return sprintf(
      // TRANSLATORS: The remaining time left on the account displayed across the app.
      // TRANSLATORS: Available placeholders:
      // TRANSLATORS: %(duration)s - a localized remaining time (in minutes, hours, or days) until the account expiry
      pgettext('account-expiry', '%(duration)s left'),
      { duration },
    );
  }
}
