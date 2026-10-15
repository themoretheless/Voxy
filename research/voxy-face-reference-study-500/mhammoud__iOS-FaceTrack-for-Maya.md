# Facial rendering source scan: mhammoud/iOS-FaceTrack-for-Maya

Repository: https://github.com/mhammoud/iOS-FaceTrack-for-Maya

Pinned source revision: bae2ead5a57bc93011303421c2c636ea645ad27f

Discovery topic: facial animation

Description: iOS FaceTrack for Maya allows you to use your iPhone X/XS/XS Max for facial motion capture for character animation in Autodesk's Maya.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["iOS FaceTrack for Maya.xcodeproj/project.pbxproj", "iOS FaceTrack for Maya.xcodeproj/project.xcworkspace/contents.xcworkspacedata", "iOS FaceTrack for Maya.xcodeproj/project.xcworkspace/xcshareddata/IDEWorkspaceChecks.plist", "iOS FaceTrack for Maya.xcodeproj/project.xcworkspace/xcuserdata/mohamedhammoud.xcuserdatad/UserInterfaceState.xcuserstate", "iOS FaceTrack for Maya.xcodeproj/xcuserdata/mohamedhammoud.xcuserdatad/xcschemes/xcschememanagement.plist", "iOS FaceTrack for Maya/AppDelegate.swift", "iOS FaceTrack for Maya/Assets.xcassets/AppIcon.appiconset/Contents.json", "iOS FaceTrack for Maya/Assets.xcassets/Contents.json", "iOS FaceTrack for Maya/Base.lproj/LaunchScreen.storyboard", "iOS FaceTrack for Maya/Base.lproj/Main.storyboard", "iOS FaceTrack for Maya/Info.plist", "iOS FaceTrack for Maya/ViewController.swift", "iOS FaceTrack for Maya/art.scnassets/ship.scn", "iOS FaceTrack for Maya/art.scnassets/texture.png", "iOS FaceTrack for MayaTests/Info.plist", "iOS FaceTrack for MayaTests/iOS_FaceTrack_for_MayaTests.swift", "iOS FaceTrack for MayaUITests/Info.plist", "iOS FaceTrack for MayaUITests/iOS_FaceTrack_for_MayaUITests.swift"]}

## Source file: iOS FaceTrack for MayaTests/iOS_FaceTrack_for_MayaTests.swift

{
  "path": "iOS FaceTrack for MayaTests/iOS_FaceTrack_for_MayaTests.swift",
  "blob_sha": "8d84f10e9b93da138a30c61332834eb97b363f5e",
  "sha256": "3cb143604271dd01ac2fcfac1cee7db613cdee6b6c9616e8466ce0424f9b9156",
  "lines": 34,
  "bytes": 973,
  "selection_score": 13,
  "keyword_line_counts": {
    "rig": 1,
    "face": 4
  },
  "url": "https://github.com/mhammoud/iOS-FaceTrack-for-Maya/blob/bae2ead5a57bc93011303421c2c636ea645ad27f/iOS%20FaceTrack%20for%20MayaTests/iOS_FaceTrack_for_MayaTests.swift",
  "function_signature_candidates": []
}

```text

//
//  iOS_FaceTrack_for_MayaTests.swift
//  iOS FaceTrack for MayaTests
//
//  Created by Mohamed Hammoud on 2019-10-11.
//  Copyright © 2019 Mohamed Hammoud. All rights reserved.
//

import XCTest
@testable import iOS_FaceTrack_for_Maya

class iOS_FaceTrack_for_MayaTests: XCTestCase {

    override func setUp() {
        // Put setup code here. This method is called before the invocation of each test method in the class.
    }

    override func tearDown() {
        // Put teardown code here. This method is called after the invocation of each test method in the class.
    }

    func testExample() {
        // This is an example of a functional test case.
        // Use XCTAssert and related functions to verify your tests produce the correct results.
    }

    func testPerformanceExample() {
        // This is an example of a performance test case.
        self.measure {
            // Put the code you want to measure the time of here.
        }
    }

}


```

## Source file: iOS FaceTrack for MayaUITests/iOS_FaceTrack_for_MayaUITests.swift

{
  "path": "iOS FaceTrack for MayaUITests/iOS_FaceTrack_for_MayaUITests.swift",
  "blob_sha": "1700a6eb595210af93847def47af2aebfdb8bad2",
  "sha256": "9a77c21e48cf88bd204f159bd02078371cfd2fc92724b5d5f2a99cac6716a44f",
  "lines": 43,
  "bytes": 1487,
  "selection_score": 13,
  "keyword_line_counts": {
    "rig": 1,
    "face": 4
  },
  "url": "https://github.com/mhammoud/iOS-FaceTrack-for-Maya/blob/bae2ead5a57bc93011303421c2c636ea645ad27f/iOS%20FaceTrack%20for%20MayaUITests/iOS_FaceTrack_for_MayaUITests.swift",
  "function_signature_candidates": []
}

```text

//
//  iOS_FaceTrack_for_MayaUITests.swift
//  iOS FaceTrack for MayaUITests
//
//  Created by Mohamed Hammoud on 2019-10-11.
//  Copyright © 2019 Mohamed Hammoud. All rights reserved.
//

import XCTest

class iOS_FaceTrack_for_MayaUITests: XCTestCase {

    override func setUp() {
        // Put setup code here. This method is called before the invocation of each test method in the class.

        // In UI tests it is usually best to stop immediately when a failure occurs.
        continueAfterFailure = false

        // In UI tests it’s important to set the initial state - such as interface orientation - required for your tests before they run. The setUp method is a good place to do this.
    }

    override func tearDown() {
        // Put teardown code here. This method is called after the invocation of each test method in the class.
    }

    func testExample() {
        // UI tests must launch the application that they test.
        let app = XCUIApplication()
        app.launch()

        // Use recording to get started writing UI tests.
        // Use XCTAssert and related functions to verify your tests produce the correct results.
    }

    func testLaunchPerformance() {
        if #available(macOS 10.15, iOS 13.0, tvOS 13.0, *) {
            // This measures how long it takes to launch your application.
            measure(metrics: [XCTOSSignpostMetric.applicationLaunch]) {
                XCUIApplication().launch()
            }
        }
    }
}


```

## Source file: iOS FaceTrack for Maya/AppDelegate.swift

{
  "path": "iOS FaceTrack for Maya/AppDelegate.swift",
  "blob_sha": "55bbe613958b541d42e36a9617af8e2d3fd616dd",
  "sha256": "73250562e202dd50c5c1297e9074dec066d97eae85f3e891250b15b3a105d3af",
  "lines": 41,
  "bytes": 1854,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 1,
    "face": 2
  },
  "url": "https://github.com/mhammoud/iOS-FaceTrack-for-Maya/blob/bae2ead5a57bc93011303421c2c636ea645ad27f/iOS%20FaceTrack%20for%20Maya/AppDelegate.swift",
  "function_signature_candidates": []
}

```text

//
//  AppDelegate.swift
//  iOS FaceTrack for Maya
//
//  Created by Mohamed Hammoud on 2019-10-11.
//  Copyright © 2019 Mohamed Hammoud. All rights reserved.
//

import UIKit

@UIApplicationMain
class AppDelegate: UIResponder, UIApplicationDelegate {

    var window: UIWindow?


    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // Override point for customization after application launch.
        return true
    }

    func applicationWillResignActive(_ application: UIApplication) {
        // Sent when the application is about to move from active to inactive state. This can occur for certain types of temporary interruptions (such as an incoming phone call or SMS message) or when the user quits the application and it begins the transition to the background state.
        // Use this method to pause ongoing tasks, disable timers, and invalidate graphics rendering callbacks. Games should use this method to pause the game.
    }

    func applicationDidEnterBackground(_ application: UIApplication) {
        // Use this method to release shared resources, save user data, invalidate timers, and store enough application state information to restore your application to its current state in case it is terminated later.
    }

    func applicationWillEnterForeground(_ application: UIApplication) {
        // Called as part of the transition from the background to the active state; here you can undo many of the changes made on entering the background.
    }

    func applicationDidBecomeActive(_ application: UIApplication) {
        // Restart any tasks that were paused (or not yet started) while the application was inactive. If the application was previously in the background, optionally refresh the user interface.
    }


}



```

## Source file: iOS FaceTrack for Maya/ViewController.swift

{
  "path": "iOS FaceTrack for Maya/ViewController.swift",
  "blob_sha": "e349ecd1f2c2b64708fd0137ddf6a5070bc005d5",
  "sha256": "b907d6be7ec4a601150842ca5e872cf8dbe5df7541d4ca51c53df901cb311123",
  "lines": 84,
  "bytes": 2529,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 1,
    "face": 10
  },
  "url": "https://github.com/mhammoud/iOS-FaceTrack-for-Maya/blob/bae2ead5a57bc93011303421c2c636ea645ad27f/iOS%20FaceTrack%20for%20Maya/ViewController.swift",
  "function_signature_candidates": []
}

```text

//
//  ViewController.swift
//  iOS FaceTrack for Maya
//
//  Created by Mohamed Hammoud on 2019-10-11.
//  Copyright © 2019 Mohamed Hammoud. All rights reserved.
//

import UIKit
import SceneKit
import ARKit

class ViewController: UIViewController, ARSCNViewDelegate {

    @IBOutlet var sceneView: ARSCNView!
    @IBOutlet weak var faceLabel: UILabel!
    @IBOutlet weak var labelView: UIView!
    
    override func viewDidLoad() {
        super.viewDidLoad()
        
        // Set the view's delegate
        sceneView.delegate = self
        
        labelView.layer.cornerRadius = 10
        
        // Show statistics such as fps and timing information
        sceneView.showsStatistics = true
        
        guard ARFaceTrackingConfiguration.isSupported else {
            fatalError("Face tracking is not supported on this device.")
        }
    }
    
    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        
        // Create a session configuration
        let configuration = ARFaceTrackingConfiguration()

        // Run the view's session
        sceneView.session.run(configuration)
    }
    
    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        
        // Pause the view's session
        sceneView.session.pause()
    }

    // MARK: - ARSCNViewDelegate

    // Override to create and configure nodes for anchors added to the view's session.
    func renderer(_ renderer: SCNSceneRenderer, nodeFor anchor: ARAnchor) -> SCNNode? {
        let faceMesh = ARSCNFaceGeometry(device: sceneView.device!)
        let node = SCNNode(geometry: faceMesh)
        node.geometry?.firstMaterial?.fillMode = .lines

        return node
    }
    
    func renderer(_ renderer: SCNSceneRenderer, didUpdate node: SCNNode, for anchor: ARAnchor) {
        let faceAnchor = anchor as? ARFaceAnchor
        let faceGeometry = node.geometry as? ARSCNFaceGeometry
        faceGeometry?.update(from: faceAnchor!.geometry)
        
    }
    
    func session(_ session: ARSession, didFailWithError error: Error) {
        // Present an error message to the user
        
    }
    
    func sessionWasInterrupted(_ session: ARSession) {
        // Inform the user that the session has been interrupted, for example, by presenting an overlay
        
    }
    
    func sessionInterruptionEnded(_ session: ARSession) {
        // Reset tracking and/or remove existing anchors if consistent tracking is required
        
    }
}


```
