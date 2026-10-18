# Doki app

Flutter front end for Doki. See [docs/app.md](../docs/app.md) for what it does, how to build it and what is verified.

- `lib/backend.dart`: the only connection to the engine (FFI on desktop, platform channel on Android).
- `lib/screens/`: Library, Incoming, History, Devices, game detail and review.
- `android/app/src/main/java/dev/doki/app/`: `MainActivity` (channel, folder picker) and `SafSaveFolder` (Storage Access Framework adapter).
