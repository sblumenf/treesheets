import 'package:flutter/material.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

/// The least a screen on a kept document can be, for tests of what is under every
/// screen: it shows the text of the root's first cell and how the keeper stands, and
/// has one field whose text goes into that cell when the keeper asks it to finish.
class ProbeScreen extends StatefulWidget {
  const ProbeScreen(this.keeper, {super.key});

  final Keeper keeper;

  @override
  State<ProbeScreen> createState() => _ProbeScreenState();
}

class _ProbeScreenState extends State<ProbeScreen> {
  final _typing = TextEditingController();
  late final Attachment _attachment;
  String? _cell;
  String _shown = '';
  String? _entered;

  /// What entering the text ran into, if anything.
  Object? _refused;

  @override
  void initState() {
    super.initState();
    _attachment = widget.keeper.attach(
      onChange: () => setState(() {}),
      finish: _enter,
    );
    _read();
  }

  /// A screen that goes enters what is typed on it: asked of the keeper before
  /// anyone can ask it to close.
  @override
  void dispose() {
    _enter();
    _attachment.detach();
    _typing.dispose();
    super.dispose();
  }

  Future<void> _read() async {
    final keeper = widget.keeper;
    final cells = (await keeper.grid(keeper.root)).cells;
    if (!mounted || cells.isEmpty) return;
    setState(() {
      _cell = cells.first.id;
      _shown = cells.first.text;
    });
  }

  /// What is typed goes into the document. What the core says to it is shown, not
  /// thrown: nothing is unfinished then.
  Future<void> _enter() async {
    final cell = _cell;
    final text = _typing.text;
    if (cell == null || text.isEmpty || text == _entered) return;
    try {
      await widget.keeper.apply(Command.enterText(cell: cell, text: text));
      _entered = text;
      await _read();
    } on BridgeError catch (error) {
      if (mounted) setState(() => _refused = error);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    body: Column(
      children: [
        Text(_shown, key: const Key('probe-shown')),
        Text(widget.keeper.state.name, key: const Key('probe-state')),
        if (_refused != null)
          Text('$_refused', key: const Key('probe-refused')),
        TextField(key: const Key('probe-typing'), controller: _typing),
      ],
    ),
  );
}
