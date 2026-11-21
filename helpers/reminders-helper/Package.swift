// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "reminders-helper",
    platforms: [.macOS(.v14)],
    targets: [
        .target(name: "RemindersCore"),
        .executableTarget(name: "RemindersHelper", dependencies: ["RemindersCore"]),
        .testTarget(name: "RemindersCoreTests", dependencies: ["RemindersCore"]),
    ],
    swiftLanguageModes: [.v5]
)
