//
//  UserDefaultsInteractor.swift
//  MullvadVPN
//
//  Created by pronebird on 15/05/2019.
//  Copyright © 2019 Amagicom AB. All rights reserved.
//

import Foundation

/// The application group identifier used for sharing application preferences between processes
private let kApplicationGroupIdentifier = "group.net.mullvad.MullvadVPN"

/// The UserDefaults keys used to store the application preferences
private enum UserDefaultsKeys: String {
    case accountToken, accountExpiry
}

/// The interactor class that provides a convenient interface for accessing the Mullvad VPN
/// preferences stored in the UserDefaults store.
class UserDefaultsInteractor {
    let userDefaults: UserDefaults

    /// The shared instance of UserDefaultsInteractor initialized with the application group
    /// preferences
    static let sharedApplicationGroupInteractor: UserDefaultsInteractor = {
        let userDefaults = UserDefaults(suiteName: kApplicationGroupIdentifier)!

        return UserDefaultsInteractor(userDefaults: userDefaults)
    }()

    init(userDefaults: UserDefaults) {
        self.userDefaults = userDefaults
    }

    var accountToken: String? {
        get {
            return userDefaults.string(forKey: UserDefaultsKeys.accountToken.rawValue)
        }
        set {
            userDefaults.set(newValue, forKey: UserDefaultsKeys.accountToken.rawValue)
        }
    }

    var accountExpiry: Date? {
        get {
            return userDefaults.object(forKey: UserDefaultsKeys.accountExpiry.rawValue) as? Date
        }
        set {
            userDefaults.set(newValue, forKey: UserDefaultsKeys.accountExpiry.rawValue)
        }
    }

}
