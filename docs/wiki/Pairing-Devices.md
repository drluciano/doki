# Pairing your devices

Your devices sync with each other directly. There is no server to set up and no account to create: every device is equal, the way Syncthing works. A phone, a handheld and a PC can each add the next device, and any two of them sync whenever both have Doki open.

## Pair two devices

1. On a device that is already set up (or your first device), open **Devices** and choose **Show a code on this device**.
2. On the new device, open **Devices**, find **Have a code from another device?**, and enter the code.

You will see **Paired with …** on the new device when it worked.

### Which code to use

Doki shows two codes. Both stay open for 15 minutes and can add any number of devices in that time.

| Code | Looks like | Works when |
|---|---|---|
| Short code | `ABCD-2345` | Both devices are on the **same Wi-Fi**. Easy to type on a handheld. |
| Long code | `DOKIP1:…` | From **any network**. Use **Copy long code** and send it to your other device however you like. |

Anyone holding a code while it is open can join your devices, so only share it with your own devices. After 20 wrong attempts, Doki closes every open code on that device.

### Adding a third device

Pair it with any device you already have. The others learn about it the next time they sync with a device that knows it, so you do not need to pair every device with every other one.

## Syncing

The app syncs:

- when it opens,
- every two minutes while it is open,
- when it comes back to the foreground,
- and when you choose **Sync now**.

Both devices need Doki running at the same time. On Android, the background watcher keeps syncing while you play (turn it off under **Devices → Keep working in the background**). On Windows, Doki keeps syncing from the tray after you close its window, until you choose **Quit**.

Games added on one device appear on the others under **On your other devices** in Library (and whole emulators under **Systems on your other devices**). Tap one and choose where it saves on this device.

**Syncing never changes a save.** Received progress waits under **Incoming** for you. See [Reviewing incoming progress](Reviewing-Incoming-Progress.md).

## Removing a device

Open **Devices**, choose the device, and choose **Remove**. The removal reaches every device on its next sync, and the removed device can no longer sync with any of yours. Checkpoints it already sent stay in your history. To add it back later, pair it again with a new code.

## Privacy

- The connection between devices is end-to-end encrypted. Each device has its own key, and a device only accepts connections from devices it trusts.
- On the same network, devices connect directly. Across networks, the connection may pass through a public relay run by the makers of [iroh](https://iroh.computer), the networking library Doki uses. That relay cannot read anything; it sees only device ids and network addresses.

## Limits

- On Android, syncing in the background needs **Keep working in the background** on. On Linux, Doki syncs only while it is open.
- The short code needs a network that allows local broadcasts. Some guest and public Wi-Fi networks block them; use the long code there.
- Syncing on a network with no internet connection at all has not been tested yet.
- QR codes are not shown yet.
- Sync is not a backup. A device lost before it syncs loses what it had not sent.

If your devices are rarely open at the same time, see [Self-hosted relay](Self-Hosted-Relay.md). Technical details are in [peer.md](https://github.com/drluciano/doki/blob/main/docs/peer.md).
