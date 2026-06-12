import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // Init VRE VM bridging here
        runVreApp("app.vym")
        
        window = UIWindow(frame: UIScreen.main.bounds)
        window?.rootViewController = UIViewController()
        window?.rootViewController?.view.backgroundColor = .white
        window?.makeKeyAndVisible()
        return true
    }
    
    // C bridge to Rust VRE Core
    func runVreApp(_ file: String) {
        // vre_ios_run(file)
    }
}
