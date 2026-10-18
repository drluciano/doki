import 'package:flutter/material.dart';

import '../motion.dart';
import '../theme.dart';
import '../widgets/common.dart';

/// One how-to: a question-shaped title and short steps or explanations.
class HelpTopic {
  const HelpTopic(this.icon, this.title, this.summary, this.points);
  final IconData icon;
  final String title;
  final String summary;
  final List<String> points;
}

/// The Help page's topics, in the order a new user needs them. The wording
/// matches the buttons and messages in the app, so a step names exactly what
/// the user will see.
const helpTopics = [
  HelpTopic(
    Icons.favorite_border,
    'How Doki keeps your saves safe',
    'Nothing replaces your save until you choose.',
    [
      'A checkpoint is a verified copy of a game\'s save. Checkpoints never change once made.',
      'Progress from your other devices arrives in Incoming. Receiving it never changes your save.',
      'Before Doki replaces a save, it backs up the current one and checks the backup.',
      'Every version stays in History, including the ones you set aside.',
    ],
  ),
  HelpTopic(
    Icons.create_new_folder_outlined,
    'Add a game',
    'Connect the folder where your emulator keeps the save.',
    [
      'In Library, choose Add game.',
      'Give it a name, then choose Connect folder and pick the game\'s save folder.',
      'Doki only reads the folder. It replaces files only when you choose Use this save or Restore.',
      'A game you added on another device shows up after a sync. Choose it and pick where it saves on this device.',
    ],
  ),
  HelpTopic(
    Icons.folder_special_outlined,
    'Track a whole emulator',
    'Add an emulator\'s save folder once, and every game in it is tracked.',
    [
      'In Add game, choose Whole system and pick the emulator\'s save folder.',
      'Each subfolder, or each set of save files sharing a name, becomes its own game. Games you play later are added on their own.',
      'Set up the same system on your other device (it shows under Systems on your other devices) to keep every game in step.',
      'Removing a game from a system stops tracking it; Track again on the system\'s page brings it back with its history.',
    ],
  ),
  HelpTopic(
    Icons.palette_outlined,
    'Game art',
    'Covers in the library and a banner on each game\'s page.',
    [
      'Open Game art in Library and paste your free SteamGridDB API key to find art automatically.',
      'Or open a game, choose Change art, and pick your own cover and banner images.',
      'Art stays on this device and never changes your saves.',
    ],
  ),
  HelpTopic(
    Icons.bookmark_add_outlined,
    'Create a checkpoint',
    'Save a copy of your progress to keep and share.',
    [
      'Close the game in the emulator first, so the save is complete.',
      'Open the game in Library and choose Create checkpoint.',
      'Add a label if you like, such as "Before the gym".',
      'Library shows "New progress not in a checkpoint yet" when you have played since the last one.',
    ],
  ),
  HelpTopic(
    Icons.devices_outlined,
    'Pair your devices',
    'Connect your PC and handheld directly. No account or server.',
    [
      'On one device, open Devices and choose Show a code on this device.',
      'On the same Wi-Fi, enter the short code on the other device. From another network, copy the long code instead.',
      'Once paired, a device also meets the rest of your devices.',
      'Remove a device in Devices any time. It can no longer sync with your devices, and checkpoints it already sent stay.',
    ],
  ),
  HelpTopic(Icons.sync, 'Sync progress', 'Send your checkpoints to your other devices.', [
    'Doki syncs while it is open on both devices. Choose Sync now in Devices to sync right away.',
    'Only checkpoints are sent. Play you have not made a checkpoint of stays on this device.',
    'Received progress waits in Incoming. Sync is not a backup.',
  ]),
  HelpTopic(
    Icons.move_to_inbox_outlined,
    'Incoming progress',
    'Saves from your other devices wait here for you.',
    [
      'A badge on Incoming shows how many are waiting.',
      'Open an incoming save to compare it with yours. Opening it changes nothing.',
    ],
  ),
  HelpTopic(
    Icons.shield_outlined,
    'Use an incoming save',
    'Choose which save to continue. You can always go back.',
    [
      'Back up and use this save makes it your current save. Your previous save becomes a checkpoint first.',
      'Keep alongside keeps the incoming save in History without using it.',
      'Keep my save leaves your save as it is. The other one stays in History.',
      'If you played on both devices, Doki says so. Both versions stay in your history.',
    ],
  ),
  HelpTopic(Icons.history, 'Restore a checkpoint', 'Go back to any earlier save.', [
    'Open the game, find the checkpoint in its history and choose Restore this checkpoint.',
    'Review it, then choose Back up and restore. Your current save is kept as a checkpoint first.',
  ]),
  HelpTopic(
    Icons.build_circle_outlined,
    'If something goes wrong',
    'Doki tells you whether your save changed and what to do next.',
    [
      'If replacing a save was interrupted, the game shows "Needs your choice: finish or put back".',
      'Finish the change completes it. Put my save back returns the save you had before.',
      'Every error has a Details section you can share if you need help.',
    ],
  ),
  HelpTopic(
    Icons.sports_esports_outlined,
    'Keyboard and controller',
    'Everything works without touch.',
    [
      'Use Tab or the D-pad to move between controls. A soft border shows where you are.',
      'Press Enter, Space or A to choose, and the back button to go back.',
    ],
  ),
];

/// Simple how-tos and how Doki works. Each topic opens in place.
class HelpScreen extends StatelessWidget {
  const HelpScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Help'), backgroundColor: Colors.transparent),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 32),
        children: [
          ContentWidth(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var i = 0; i < helpTopics.length; i++)
                  Entrance(
                    delay: Duration(milliseconds: 30 * i),
                    child: HelpTopicCard(topic: helpTopics[i], initiallyOpen: i == 0),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class HelpTopicCard extends StatefulWidget {
  const HelpTopicCard({super.key, required this.topic, this.initiallyOpen = false});
  final HelpTopic topic;
  final bool initiallyOpen;

  @override
  State<HelpTopicCard> createState() => _HelpTopicCardState();
}

class _HelpTopicCardState extends State<HelpTopicCard> {
  late bool _open = widget.initiallyOpen;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final topic = widget.topic;
    final reduced = DokiMotion.reduced(context);
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: () => setState(() => _open = !_open),
        child: Semantics(
          expanded: _open,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Container(
                      width: 40,
                      height: 40,
                      decoration: BoxDecoration(
                        color: DokiColors.lavenderSoft,
                        borderRadius: BorderRadius.circular(14),
                      ),
                      child: Icon(topic.icon, color: DokiColors.lavender),
                    ),
                    const SizedBox(width: 14),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(topic.title, style: text.titleMedium),
                          const SizedBox(height: 2),
                          Text(
                            topic.summary,
                            style: text.bodyMedium?.copyWith(color: DokiColors.muted),
                          ),
                        ],
                      ),
                    ),
                    AnimatedRotation(
                      turns: _open ? 0.5 : 0,
                      duration: reduced ? Duration.zero : DokiMotion.panel,
                      curve: DokiMotion.enter,
                      child: const Icon(Icons.expand_more, color: DokiColors.muted),
                    ),
                  ],
                ),
                AnimatedSize(
                  duration: reduced ? Duration.zero : DokiMotion.panel,
                  curve: DokiMotion.enter,
                  alignment: Alignment.topCenter,
                  child: AnimatedOpacity(
                    opacity: _open ? 1 : 0,
                    duration: DokiMotion.quick,
                    child: _open
                        ? Padding(
                            padding: const EdgeInsets.only(top: 12, left: 54),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                for (final p in topic.points)
                                  Padding(
                                    padding: const EdgeInsets.only(bottom: 8),
                                    child: Row(
                                      crossAxisAlignment: CrossAxisAlignment.start,
                                      children: [
                                        const Padding(
                                          padding: EdgeInsets.only(top: 7, right: 10),
                                          child: Icon(
                                            Icons.circle,
                                            size: 6,
                                            color: DokiColors.lavender,
                                          ),
                                        ),
                                        Expanded(child: Text(p, style: text.bodyLarge)),
                                      ],
                                    ),
                                  ),
                              ],
                            ),
                          )
                        : const SizedBox(width: double.infinity),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
