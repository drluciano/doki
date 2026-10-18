import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:path_provider/path_provider.dart';

import 'appearance.dart';
import 'backend.dart';
import 'motion.dart';
import 'screens/devices.dart';
import 'screens/settings.dart';
import 'screens/history.dart';
import 'screens/incoming.dart';
import 'screens/library.dart';
import 'screens/splash.dart';
import 'screens/welcome.dart';
import 'state.dart';
import 'theme.dart';
import 'widgets/focus_halo.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final backend = await DokiBackend.forPlatform();
  final state = AppState(backend, autoSync: true, autoCapture: true)..refresh();
  if (Platform.isWindows) {
    // "Sync now" in the tray menu (windows/runner/flutter_window.cpp).
    const MethodChannel('dev.doki/window').setMethodCallHandler((call) async {
      if (call.method == 'sync') await state.sync().catchError((_) => null);
    });
  }
  final support = await getApplicationSupportDirectory();
  final appearance = await Appearance.load(
    File('${support.path}${Platform.pathSeparator}appearance.json'),
  );
  runApp(DokiApp(state: state, appearance: appearance));
}

/// Makes [AppState] available to every screen.
class AppScope extends InheritedNotifier<AppState> {
  const AppScope({super.key, required AppState state, required super.child})
    : super(notifier: state);

  static AppState of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<AppScope>()!.notifier!;

  static AppState read(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AppScope>()!.notifier!;
}

class DokiApp extends StatefulWidget {
  const DokiApp({super.key, required this.state, this.appearance});
  final AppState state;

  /// Theme and border color. Without one (tests), the defaults are used and
  /// changes are kept in memory only.
  final Appearance? appearance;

  @override
  State<DokiApp> createState() => _DokiAppState();
}

class _DokiAppState extends State<DokiApp> {
  late final Appearance _appearance = widget.appearance ?? Appearance();

  @override
  Widget build(BuildContext context) {
    return AppScope(
      state: widget.state,
      child: AppearanceScope(
        appearance: _appearance,
        child: ListenableBuilder(
          listenable: _appearance,
          builder: (context, _) {
            final dark = _appearance.darkPalette;
            return MaterialApp(
              title: 'Doki',
              theme: dokiTheme(DokiPalette.light),
              darkTheme: dokiTheme(dark),
              themeMode: _appearance.themeMode,
              // Switch palettes in one step: Doki's own colors change at
              // once, so a cross-fade of only the Material parts would look
              // half-finished.
              themeAnimationDuration: Duration.zero,
              debugShowCheckedModeBanner: false,
              navigatorObservers: [FocusHalo.observer],
              builder: (context, child) => PaletteSync(
                darkPalette: dark,
                child: FocusHalo(
                  color: _appearance.accent.resolve(Theme.of(context).brightness),
                  child: child!,
                ),
              ),
              home: const SplashGate(child: RootGate()),
            );
          },
        ),
      ),
    );
  }
}

/// Shows onboarding until the vault exists, then the main shell.
class RootGate extends StatelessWidget {
  const RootGate({super.key});

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    if (!state.loaded) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    if (!state.initialized) return const WelcomeScreen();
    return const Shell();
  }
}

class Shell extends StatefulWidget {
  const Shell({super.key});

  @override
  State<Shell> createState() => _ShellState();
}

class _ShellState extends State<Shell> {
  int _index = 0;

  static const _destinations = [
    (Icons.videogame_asset_outlined, Icons.videogame_asset, 'Library'),
    (Icons.move_to_inbox_outlined, Icons.move_to_inbox, 'Incoming'),
    (Icons.history_outlined, Icons.history, 'History'),
    (Icons.devices_outlined, Icons.devices, 'Devices'),
    (Icons.settings_outlined, Icons.settings, 'Settings'),
  ];

  @override
  Widget build(BuildContext context) {
    final state = AppScope.of(context);
    final waiting = state.incoming.length + state.unmapped.length;
    final pages = [
      LibraryScreen(onOpenIncoming: () => setState(() => _index = 1)),
      const IncomingScreen(),
      const HistoryScreen(),
      const DevicesScreen(),
      const SettingsScreen(),
    ];
    Widget icon(int i, bool selected) {
      final d = _destinations[i];
      final base = Icon(selected ? d.$2 : d.$1);
      return i == 1 && waiting > 0 ? Badge(label: Text('$waiting'), child: base) : base;
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= 720;
        final body = FadeIndexedStack(index: _index, children: pages);
        if (!wide) {
          return Scaffold(
            body: body,
            bottomNavigationBar: NavigationBar(
              selectedIndex: _index,
              onDestinationSelected: (i) => setState(() => _index = i),
              destinations: [
                for (var i = 0; i < _destinations.length; i++)
                  NavigationDestination(
                    icon: icon(i, false),
                    selectedIcon: icon(i, true),
                    label: _destinations[i].$3,
                  ),
              ],
            ),
          );
        }
        return Scaffold(
          body: Row(
            children: [
              NavigationRail(
                selectedIndex: _index,
                onDestinationSelected: (i) => setState(() => _index = i),
                labelType: NavigationRailLabelType.all,
                leading: Padding(
                  padding: EdgeInsets.symmetric(vertical: 16),
                  child: Text(
                    'doki',
                    style: TextStyle(
                      fontSize: 22,
                      fontWeight: FontWeight.w800,
                      color: DokiColors.lavender,
                    ),
                  ),
                ),
                destinations: [
                  for (var i = 0; i < _destinations.length; i++)
                    NavigationRailDestination(
                      icon: icon(i, false),
                      selectedIcon: icon(i, true),
                      label: Text(_destinations[i].$3),
                    ),
                ],
              ),
              const VerticalDivider(width: 1),
              Expanded(child: body),
            ],
          ),
        );
      },
    );
  }
}
