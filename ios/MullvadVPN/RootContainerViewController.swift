//
//  RootContainerViewController.swift
//  MullvadVPN
//
//  Created by pronebird on 25/05/2019.
//  Copyright © 2019 Amagicom AB. All rights reserved.
//

import UIKit

enum HeaderBarStyle {
    case transparent, `default`, unsecured, secured

    fileprivate func backgroundColor() -> UIColor {
        switch self {
        case .transparent:
            return UIColor.clear
        case .default:
            return UIColor.HeaderBar.defaultBackgroundColor
        case .secured:
            return UIColor.HeaderBar.securedBackgroundColor
        case .unsecured:
            return UIColor.HeaderBar.unsecuredBackgroundColor
        }
    }
}

/// A protocol that defines the relationship between the root container and its child controllers
protocol RootContainment {

    /// Return the preferred header bar style
    var preferredHeaderBarStyle: HeaderBarStyle { get }

}

/// A root container class that primarily handles the unwind storyboard segues on log out
class RootContainerViewController: UIViewController {

    typealias CompletionHandler = () -> Void

    private var viewControllers = [UIViewController]()

    private var topViewController: UIViewController? {
        return viewControllers.last
    }

    @IBOutlet var headerBarView: UIView!
    @IBOutlet var headerBarSettingsButton: UIButton!
    @IBOutlet var transitionContainer: UIView!

    private(set) var headerBarStyle = HeaderBarStyle.default

    override var childForStatusBarStyle: UIViewController? {
        return topViewController
    }

    override var childForStatusBarHidden: UIViewController? {
        return topViewController
    }

    override var shouldAutomaticallyForwardAppearanceMethods: Bool {
        return false
    }

    // MARK: - View lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()

        var margins = view.layoutMargins
        margins.left = 24
        margins.right = 24
        view.layoutMargins = margins

        updateHeaderBarBackground()
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()

        updateAdditionalSafeAreaInsetsIfNeeded()
    }

    override func viewSafeAreaInsetsDidChange() {
        super.viewSafeAreaInsetsDidChange()

        updateHeaderBarLayoutMarginsIfNeeded()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)

        topViewController?.beginAppearanceTransition(true, animated: animated)
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)

        topViewController?.endAppearanceTransition()
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)

        topViewController?.beginAppearanceTransition(false, animated: animated)
    }

    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)

        topViewController?.endAppearanceTransition()
    }

    // MARK: - Storyboard segue handling

    override func unwind(for unwindSegue: UIStoryboardSegue, towards subsequentVC: UIViewController) {
        let index = viewControllers.firstIndex(of: subsequentVC)!
        let newViewControllers = Array(viewControllers.prefix(through: index))

        let animated = UIView.areAnimationsEnabled

        setViewControllers(newViewControllers, animated: animated)
    }

    // MARK: - Public

    func setViewControllers(_ newViewControllers: [UIViewController], animated: Bool, completion: CompletionHandler? = nil) {
        // Dot not handle appearance events when the container itself is not visible
        let shouldHandleAppearanceEvents = view.window != nil

        // Animations won't run when the container is not visible, so prevent them
        let shouldAnimate = animated && shouldHandleAppearanceEvents

        let sourceViewController = topViewController
        let targetViewController = newViewControllers.last

        let viewControllersToAdd = newViewControllers.filter { !viewControllers.contains($0) }
        let viewControllersToRemove = viewControllers.filter { !newViewControllers.contains($0) }

        let finishTransition = {
            // Notify the added controllers that they finished a transition into the container
            for child in viewControllersToAdd {
                child.didMove(toParent: self)
            }

            // Remove the controllers that transitioned out of the container
            // The call to removeFromParent() automatically calls child.didMove()
            for child in viewControllersToRemove {
                child.view.removeFromSuperview()
                child.removeFromParent()
            }

            // Remove the source controller from view hierarchy
            if sourceViewController != targetViewController {
                sourceViewController?.view.removeFromSuperview()
            }

            // Finish appearance transition
            if shouldHandleAppearanceEvents {
                sourceViewController?.endAppearanceTransition()
                if sourceViewController != targetViewController {
                    targetViewController?.endAppearanceTransition()
                }
            }

            completion?()
        }

        let alongSideAnimations = {
            self.updateHeaderBarStyleFromChildPreferences(animated: shouldAnimate)
        }

        // Make sure that all new view controllers have loaded their views
        // This is important because the unwind segue calls the unwind action which may rely on
        // IB outlets to be set at that time.
        for newViewController in newViewControllers {
            newViewController.loadViewIfNeeded()
        }

        // Add new child controllers. The call to addChild() automatically calls child.willMove()
        // Children have to be registered in the container for Storyboard unwind segues to function
        // properly, however the child controller views don't have to be added immediately, and
        // appearance methods have to be handled manually.
        for child in viewControllersToAdd {
            addChild(child)
        }

        // Add the destination view into the view hierarchy
        if let targetView = targetViewController?.view {
            addChildView(targetView)
        }

        // Notify the controllers that they will transition out of the container
        for child in viewControllersToRemove {
            child.willMove(toParent: nil)
        }

        viewControllers = newViewControllers

        // Begin appearance transition
        if shouldHandleAppearanceEvents {
            sourceViewController?.beginAppearanceTransition(false, animated: shouldAnimate)
            if sourceViewController != targetViewController {
                targetViewController?.beginAppearanceTransition(true, animated: shouldAnimate)
            }
        }

        if shouldAnimate {
            CATransaction.begin()
            CATransaction.setCompletionBlock {
                finishTransition()
            }

            let transition = CATransition()
            transition.duration = 0.35
            transition.type = .push

            // Pick the animation movement direction
            let sourceIndex = sourceViewController.flatMap({ newViewControllers.firstIndex(of: $0) })
            let targetIndex = targetViewController.flatMap({ newViewControllers.firstIndex(of: $0) })

            switch (sourceIndex, targetIndex) {
            case (.some(let lhs), .some(let rhs)):
                transition.subtype = lhs > rhs ?  .fromLeft : .fromRight
            case (.none, .some):
                transition.subtype = .fromLeft
            default:
                transition.subtype = .fromRight
            }

            transitionContainer.layer.add(transition, forKey: "transition")
            alongSideAnimations()

            CATransaction.commit()
        } else {
            alongSideAnimations()
            finishTransition()
        }
    }

    func pushViewController(_ viewController: UIViewController, animated: Bool) {
        var newViewControllers = viewControllers.filter({ $0 != viewController })
        newViewControllers.append(viewController)

        setViewControllers(newViewControllers, animated: animated)
    }

    /// Request the root container to query the top controller for the new header bar style
    func setNeedsHeaderBarStyleAppearance() {
        updateHeaderBarStyleFromChildPreferences(animated: UIView.areAnimationsEnabled)
    }

    // MARK: - Actions

    @IBAction func doShowSettings() {
        performSegue(withIdentifier: SegueIdentifier.Root.showSettings.rawValue, sender: self)
    }

    // MARK: - Private

    private func addChildView(_ childView: UIView) {
        childView.translatesAutoresizingMaskIntoConstraints = true
        childView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        childView.frame = transitionContainer.bounds

        transitionContainer.addSubview(childView)
    }

    /// Updates the header bar's layout margins to make sure it doesn't go below the system status
    /// bar.
    private func updateHeaderBarLayoutMarginsIfNeeded() {
        let offsetTop = view.safeAreaInsets.top - additionalSafeAreaInsets.top

        var layoutMargins = headerBarView.layoutMargins
        layoutMargins.top = offsetTop

        if layoutMargins != headerBarView.layoutMargins {
            headerBarView.layoutMargins = layoutMargins
        }
    }

    /// Updates additional safe area insets to push the child views below the header bar
    private func updateAdditionalSafeAreaInsetsIfNeeded() {
        var safeAreaInstes = additionalSafeAreaInsets
        safeAreaInstes.top = headerBarView.frame.height

        if additionalSafeAreaInsets != safeAreaInstes {
            additionalSafeAreaInsets = safeAreaInstes
        }
    }

    private func setHeaderBarStyle(_ style: HeaderBarStyle, animated: Bool) {
        headerBarStyle = style

        let action = {
            self.updateHeaderBarBackground()
        }

        if animated {
            UIView.animate(withDuration: 0.25, animations: action)
        } else {
            action()
        }
    }

    private func updateHeaderBarBackground() {
        headerBarView.backgroundColor = headerBarStyle.backgroundColor()
    }

    private func updateHeaderBarStyleFromChildPreferences(animated: Bool) {
        if let conforming = topViewController as? RootContainment {
            setHeaderBarStyle(conforming.preferredHeaderBarStyle, animated: animated)
        }
    }

}

class RootContainerPushSegue: UIStoryboardSegue {
    override func perform() {
        let container = source.rootContainerController!
        let animated = UIView.areAnimationsEnabled

        container.pushViewController(destination, animated: animated)
    }
}

extension UIViewController {

    var rootContainerController: RootContainerViewController? {
        var viewController: UIViewController? = parent

        while viewController != nil {
            if let container = viewController as? RootContainerViewController {
                return container
            }

            viewController = viewController?.parent
        }

        return nil
    }

}
