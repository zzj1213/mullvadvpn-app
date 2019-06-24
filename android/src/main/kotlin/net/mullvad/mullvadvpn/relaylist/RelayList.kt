package net.mullvad.mullvadvpn.relaylist

import net.mullvad.mullvadvpn.model.Constraint
import net.mullvad.mullvadvpn.model.LocationConstraint

class RelayList {
    val countries: List<RelayCountry>

    constructor(model: net.mullvad.mullvadvpn.model.RelayList) {
        countries = model.countries
            .map { country ->
                val cities = country.cities
                    .map { city -> 
                        val relays = city.relays
                            .filter { relay -> relay.hasWireguardTunnels }
                            .map { relay -> Relay(country.code, city.code, relay.hostname) }

                        RelayCity(city.name, country.code, city.code, false, relays)
                    }
                    .filter { city -> city.relays.isNotEmpty() }

                RelayCountry(country.name, country.code, false, cities)
            }
            .filter { country -> country.cities.isNotEmpty() }
    }

    fun findItemForLocation(
        constraint: Constraint<LocationConstraint>,
        expand: Boolean = false
    ): RelayItem? {
        when (constraint) {
            is Constraint.Any -> return null
            is Constraint.Only -> {
                val location = constraint.value

                when (location) {
                    is LocationConstraint.Country -> {
                        return countries.find { country -> country.code == location.countryCode }
                    }
                    is LocationConstraint.City -> {
                        val country = countries.find { country ->
                            country.code == location.countryCode
                        }

                        if (expand) {
                            country?.expanded = true
                        }

                        return country?.cities?.find { city -> city.code == location.cityCode }
                    }
                    is LocationConstraint.Hostname -> {
                        val country = countries.find { country ->
                            country.code == location.countryCode
                        }

                        val city = country?.cities?.find { city -> city.code == location.cityCode }

                        if (expand) {
                            country?.expanded = true
                            city?.expanded = true
                        }

                        return city?.relays?.find { relay -> relay.name == location.hostname }
                    }
                }
            }
        }
    }
}
