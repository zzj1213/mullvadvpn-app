//
//  StaticTableViewDataSource.swift
//  MullvadVPN
//
//  Created by pronebird on 24/05/2019.
//  Copyright © 2019 Amagicom AB. All rights reserved.
//

import UIKit

class StaticTableViewRow {
    typealias ConfigurationBlock = (IndexPath, UITableViewCell) -> Void
    typealias ActionBlock = (IndexPath) -> Void

    let reuseIdentifier: String
    let configurationBlock: ConfigurationBlock

    var isSelectable = true
    var isHidden = false
    var actionBlock: ActionBlock?

    init(reuseIdentifier: String, configurationBlock: @escaping ConfigurationBlock) {
        self.reuseIdentifier = reuseIdentifier
        self.configurationBlock = configurationBlock
    }
}

class StaticTableViewSection {
    private(set) var rows = [StaticTableViewRow]()

    var isHidden: Bool {
        return rows.allSatisfy({ $0.isHidden })
    }

    func addRows(_ rows: [StaticTableViewRow]) {
        self.rows.append(contentsOf: rows)
    }
}

class StaticTableViewDataSource: NSObject, UITableViewDataSource, UITableViewDelegate {

    private(set) var sections = [StaticTableViewSection]()

    func addSections(_ sections: [StaticTableViewSection]) {
        self.sections.append(contentsOf: sections)
    }

    // MARK: - UITableViewDelegate

    func tableView(_ tableView: UITableView, shouldHighlightRowAt indexPath: IndexPath) -> Bool {
        let row = self.row(for: indexPath)

        return row.isSelectable
    }

    // MARK: - UITableViewDataSource

    func numberOfSections(in tableView: UITableView) -> Int {
        return sections.reduce(0, { $1.isHidden ? $0 : $0 + 1 })
    }

    func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
        return sections[section].rows.count
    }

    func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
        let row = self.row(for: indexPath)
        let reuseIdentifier = row.reuseIdentifier

        let cell = tableView.dequeueReusableCell(withIdentifier: reuseIdentifier, for: indexPath)

        row.configurationBlock(indexPath, cell)

        return cell
    }

    // MARK: - Private

    private func row(for indexPath: IndexPath) -> StaticTableViewRow {
        let section = self.section(for: indexPath)
        let row = section.rows.compactMap({ $0.isHidden ? nil : $0 })

        return row[indexPath.row]
    }

    private func section(for indexPath: IndexPath) -> StaticTableViewSection {
        let visibleSections = sections.compactMap({ $0.isHidden ? nil : $0 })

        return visibleSections[indexPath.section]
    }

}
