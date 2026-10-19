# Device-to-device sync

Your devices sync with each other directly. There is no host, server or account: every device is equal, the way Syncthing works. A phone, a handheld and a PC can each pair new devices, and any two of them sync whenever both are online.

The code is in `crates/doki-peer`. The app uses it from **Devices**; the command line uses `doki devices`. The [relay](relay.md) still exists as an optional extra for devices that are rarely online at the same time.

## Pairing

1. On any device already in your group (or your first device), choose **Show a code on this device** (`doki devices invite`).
2. On the new device, enter the code under **Have a code from another device?** (`doki devices join CODE`).

There are two codes, open for 15 minutes for any number of devices:

- The **short code** (`ABCD-2345`) is easy to type on a handheld with no camera. It only works when both devices are on the same network: the joining device finds the one showing a code by a broadcast on the local network (UDP port 47100), then proves it knows the code over the encrypted connection. The broadcast reply carries only the device's id and addresses, never the code.
- The **long code** (`DOKIP1:…`) works from any network. Copy it to the other device however you like.

Anyone holding a code while it is open can join your group, so show codes only to your own devices. After 20 wrong attempts, Doki closes every open code on that device.

Pairing makes the new device known to the whole group. Each device has its own key (an Ed25519 key pair kept in the vault), and every device keeps a book of the devices it trusts. When two devices sync, they share their books, so a device paired through your handheld is trusted by your PC after the PC next syncs with any device that knows it.

## Syncing

The app syncs when it opens, every two minutes while it is open (on Android also from the background watcher, on Windows also from the tray), when it returns to the foreground, and when you press **Sync now**. The command line syncs on `doki devices sync`; `doki devices serve` keeps answering other devices until stopped.

Each sync with a device:

1. Opens an end-to-end encrypted connection (QUIC through [iroh](https://iroh.computer)) and checks that the other device is in this device's book. Unknown devices are refused.
2. Exchanges device books and the list of checkpoints each side has.
3. Sends a handoff package (the same format as `doki send`) for each game whose current save the other device does not have, with its history.
4. Lists games the other device has set up and this one has not, so you can set them up here with one tap (Library → **On your other devices**), even before they have a checkpoint. Whole-system folders are listed too (Library → **Systems on your other devices**), with which games belong to each; once a system is set up on both devices, its games are set up on their own, and their progress waits in Incoming (see the wiki page *Tracking a whole emulator*). Older devices ignore these fields.
5. Imports what arrived into the vault. **Receiving never changes a save**: received progress waits in Incoming for you to review, exactly as with a handoff file (safety contract, section 5).

Devices reach each other directly on the same network. Across networks, iroh tries to connect them directly through NAT and otherwise forwards the encrypted traffic through a public relay run by n0 (iroh's maintainers), which cannot read it. Discovery of a device's current address also goes through n0's public service and reveals only device ids and network addresses.

## Removing a device

**Remove** (`doki devices remove NAME`) takes a device out of the group. The removal spreads to every device on its next sync, and no device re-adds it from someone else's book. The removed device can no longer sync with any of yours. Checkpoints it already sent stay in history. To add it back, pair it again with a new code.

## Limits

- Both devices must be online, with Doki open, at the same time. Use the relay if that rarely happens.
- The short code needs both devices on the same network, and a network that allows local broadcasts. Some guest and public Wi-Fi networks block them; use the long code there.
- On Android, background syncing needs the background watcher (on by default, with a notification). With it off, Doki syncs only while the app is open. On Windows Doki keeps syncing from the tray after its window is closed, until you choose **Quit**; turn on **Start Doki with Windows** to have it running after every sign-in. On Linux Doki syncs while it is open.
- Syncing on a network with no internet connection at all has not been tested. Devices remember each other's local addresses, but finding a device whose address changed needs the internet until local discovery (mDNS) is built.
- QR codes are not built yet.
- Sync is not a backup. A device that is lost before it syncs loses what it had not sent.
