// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "PrivateWhisper",
    platforms: [.macOS(.v14)],
    dependencies: [
        // WhisperKit (Argmax OSS): CoreML/ANE движок, нативно поддерживает large-v3-turbo.
        .package(url: "https://github.com/argmaxinc/argmax-oss-swift.git", from: "1.0.0")
    ],
    targets: [
        .target(name: "PrivateWhisperCore"),
        .executableTarget(
            name: "PrivateWhisperApp",
            dependencies: [
                "PrivateWhisperCore",
                .product(name: "WhisperKit", package: "argmax-oss-swift")
            ]
        ),
        .testTarget(
            name: "PrivateWhisperCoreTests",
            dependencies: ["PrivateWhisperCore"]
        )
    ]
)
