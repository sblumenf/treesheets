import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../session.dart';
import 'log.dart';
import 'variants.dart';
import 'words.dart';

/// The sheet of the prototype: its variant switches and thresholds (UI_SPEC
/// sections 3 and 6) and its log (section 16), which can be written to a file or
/// put on the clipboard to be sent on.
///
/// Keys: `settings-close`; `variant-tap-text`, `variant-tap-child`, `variant-pin`; `min-text` with
/// `min-text-less` and `min-text-more`; `preview-scale` with `preview-scale-less`
/// and `preview-scale-more`; `log-count`, `log-export`, `log-copy`, `log-said`.
class SettingsSheet extends StatefulWidget {
  const SettingsSheet({
    super.key,
    required this.variants,
    required this.log,
    required this.exportFolder,
  });

  final ValueNotifier<Variants> variants;
  final ProtoLog log;

  /// Where an exported log goes; null where there is no such place, and then the
  /// log can only be copied.
  final Future<Directory> Function()? exportFolder;

  @override
  State<SettingsSheet> createState() => _SettingsSheetState();
}

class _SettingsSheetState extends State<SettingsSheet> {
  /// What the last export or copy had to say.
  String? _said;

  /// Changes the settings as they are when the tap comes, which is not always
  /// as they were when the sheet was last drawn.
  void _set(Variants Function(Variants now) change) {
    final to = change(widget.variants.value);
    widget.variants.value = to;
    widget.log.add('variant', to);
  }

  Future<void> _export(Future<Directory> Function() folder) async {
    String said;
    try {
      final file = await widget.log.export(await folder());
      said = Words.logExported(file.path);
    } catch (error) {
      said = Words.logNotExported(describeError(error));
    }
    if (mounted) setState(() => _said = said);
  }

  Future<void> _copy() async {
    await Clipboard.setData(ClipboardData(text: widget.log.text));
    if (mounted) setState(() => _said = Words.logCopied);
  }

  Widget _stepper(
    String key,
    String label,
    VoidCallback? less,
    VoidCallback? more,
  ) => ListTile(
    title: Text(label, key: Key(key)),
    trailing: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        IconButton(
          key: Key('$key-less'),
          tooltip: Words.less,
          icon: const Icon(Icons.remove),
          onPressed: less,
        ),
        IconButton(
          key: Key('$key-more'),
          tooltip: Words.more,
          icon: const Icon(Icons.add),
          onPressed: more,
        ),
      ],
    ),
  );

  @override
  Widget build(BuildContext context) => SafeArea(
    child: SingleChildScrollView(
      child: ListenableBuilder(
        // Every entry the log gets while the sheet is open is a change made here.
        listenable: widget.variants,
        builder: (context, _) {
          final v = widget.variants.value;
          final export = widget.exportFolder;
          final said = _said;
          return Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              ListTile(
                title: Text(
                  Words.settings,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                // On a small screen the sheet covers everything, and there is
                // nothing beside it to tap.
                trailing: IconButton(
                  key: const Key('settings-close'),
                  tooltip: Words.close,
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ),
              SwitchListTile(
                key: const Key('variant-tap-text'),
                title: const Text(Words.tapTextSelects),
                subtitle: const Text(Words.tapTextSelectsOff),
                value: v.tapTextSelects,
                onChanged: (on) => _set((v) => v.copyWith(tapTextSelects: on)),
              ),
              SwitchListTile(
                key: const Key('variant-tap-child'),
                title: const Text(Words.tapChildUnfolds),
                subtitle: const Text(Words.tapChildUnfoldsOff),
                value: v.tapChildUnfolds,
                onChanged: (on) => _set((v) => v.copyWith(tapChildUnfolds: on)),
              ),
              SwitchListTile(
                key: const Key('variant-pin'),
                title: const Text(Words.pin),
                value: v.pin,
                onChanged: (on) => _set((v) => v.copyWith(pin: on)),
              ),
              _stepper(
                'min-text',
                Words.minText(v.minText),
                v.minText > Variants.minTextRange.least
                    ? () => _set((v) => v.copyWith(minText: v.minText - 1))
                    : null,
                v.minText < Variants.minTextRange.most
                    ? () => _set((v) => v.copyWith(minText: v.minText + 1))
                    : null,
              ),
              _stepper(
                'preview-scale',
                Words.previewScale(v.previewScale),
                v.previewScale > Variants.previewScaleRange.least + 0.01
                    ? () => _set(
                        (v) => v.copyWith(
                          previewScale: _tenths(v.previewScale - 0.1),
                        ),
                      )
                    : null,
                v.previewScale < Variants.previewScaleRange.most - 0.01
                    ? () => _set(
                        (v) => v.copyWith(
                          previewScale: _tenths(v.previewScale + 0.1),
                        ),
                      )
                    : null,
              ),
              const Divider(),
              ListTile(
                title: Text(
                  Words.logCount(widget.log.lines.length),
                  key: const Key('log-count'),
                ),
              ),
              if (export != null)
                ListTile(
                  key: const Key('log-export'),
                  leading: const Icon(Icons.save_alt),
                  title: const Text(Words.exportLog),
                  onTap: () => _export(export),
                ),
              ListTile(
                key: const Key('log-copy'),
                leading: const Icon(Icons.copy),
                title: const Text(Words.copyLog),
                onTap: _copy,
              ),
              if (said != null)
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: SelectableText(said, key: const Key('log-said')),
                ),
            ],
          );
        },
      ),
    ),
  );
}

/// `value` to one place after the point: steps of a tenth do not add up exactly.
double _tenths(double value) => (value * 10).round() / 10;
