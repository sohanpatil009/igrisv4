You are a senior Rust systems architect.

You are working on IGRIS v4, a cross-platform AI desktop assistant written in Rust using Dioxus.

Your goal is to design and implement the first version of a new subsystem called the "Desktop Ecosystem".

This subsystem will eventually power:

- Universal Clipboard
- Cross-device notifications
- Device discovery
- Shared AI memory
- Session handoff
- Cross-device command execution
- Device synchronization
- Remote application launch
- Distributed task execution
- Ecosystem dashboard

This is NOT a standalone feature.

Design it as a reusable platform that future ecosystem features can plug into.

==================================================
IMPORTANT ARCHITECTURE RULES
==================================================

Create a new folder:

src/
    eco/

This folder must contain ONLY the ecosystem core logic.

DO NOT place UI inside eco.

DO NOT place platform-specific code inside eco.

DO NOT place networking code directly into unrelated folders.

The eco module should expose a clean API that the rest of IGRIS can use.

==================================================
Folder Structure
==================================================

Design something similar to:

src/
    eco/
        mod.rs

        manager.rs
            Main ecosystem controller.

        protocol.rs
            Packet definitions.
            Versioning.
            Serialization.

        discovery.rs
            Device discovery.
            Device registration.
            Heartbeats.

        clipboard.rs
            Universal clipboard logic.

        sync.rs
            Synchronization manager.

        device.rs
            Device representation.

        events.rs
            Internal event system.

        permissions.rs
            Trust and permission system.

        storage.rs
            Persistent ecosystem database.

        crypto.rs
            Encryption helpers.

        transport.rs
            High level networking abstraction.

        errors.rs

        config.rs

        constants.rs

==================================================
Platform Specific Code
==================================================

Platform specific implementations must NOT go into eco.

Instead create

platform/

    ecosystem/

        windows.rs

        linux.rs

        macos.rs

These files should contain

- clipboard access
- notifications
- session APIs
- native integrations

eco must call traits instead of platform APIs directly.

==================================================
Networking
==================================================

Reuse the FastSwap networking concepts whenever possible.

Avoid duplicating networking code.

Instead expose reusable networking interfaces.

The ecosystem should support:

- local LAN
- WiFi
- hotspot
- ethernet

Internet should NOT be required.

Future support for internet relay should be possible without redesign.

==================================================
Universal Clipboard
==================================================

Implement Universal Clipboard first.

Support:

- text
- images
- file metadata
- multiple clipboard formats

Clipboard changes should automatically synchronize between trusted devices.

Clipboard synchronization should avoid infinite loops.

Implement:

ClipboardChanged

ClipboardReceived

ClipboardApplied

events.

==================================================
Device Discovery
==================================================

Implement automatic discovery.

Devices should broadcast:

UUID

Device Name

Platform

Hostname

Capabilities

Version

Public Key

Battery (future)

Status

Each device should maintain a live device list.

Offline devices should disappear after timeout.

==================================================
Security
==================================================

Never trust unknown devices.

Implement pairing.

Each device has:

UUID

Public Key

Private Key

Trusted Device List

Every message should be signed.

Encryption should be abstracted so stronger algorithms can be added later.

==================================================
Event Driven Architecture
==================================================

Everything should communicate through events.

Examples:

DeviceDiscovered

ClipboardChanged

ClipboardSynced

DeviceConnected

DeviceDisconnected

PermissionGranted

PermissionDenied

Future modules must only subscribe to events.

Avoid tight coupling.

==================================================
Persistence
==================================================

Store ecosystem state inside:

pkg/ecosystem/

Store:

Known devices

Trusted devices

Clipboard history

Pairing information

Settings

Do not use hardcoded paths.

==================================================
Integration
==================================================

Integrate cleanly with:

FastSwap

Plugin system

Voice assistant

Settings

Startup manager

Config manager

The ecosystem should initialize during startup.

Expose only one public manager.

Example:

EcoManager::initialize()

EcoManager::start()

EcoManager::shutdown()

==================================================
Future Ready
==================================================

The architecture must already support future features without redesign:

Universal Clipboard

Notification Sync

Shared AI Memory

Remote Commands

Shared Search

Session Handoff

Application Sync

Distributed AI

Cross-device Automation

Universal Hotkeys

Shared Downloads

Media Routing

Device Dashboard

==================================================
Coding Style
==================================================

Follow the architecture already used in IGRIS.

Keep files small.

Avoid large monolithic modules.

Use traits whenever platform behavior differs.

Avoid duplicated logic.

Document all public APIs.

Write idiomatic Rust.

Prefer async Tokio tasks where appropriate.

Design for maintainability instead of shortcuts.

==================================================
IMPORTANT
==================================================

Do NOT implement every future feature now.

Only build the reusable ecosystem foundation plus the complete Universal Clipboard implementation.

Everything else should only have interfaces and extension points.

The finished architecture should allow adding new ecosystem features with minimal modifications.