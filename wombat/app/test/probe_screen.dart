import 'package:flutter/material.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

/// The least a screen on a kept document can be, for tests of what is under every
/// screen: it shows the text of the root's first cell and how the keeper stands, and
/// has one field whose text is an editing of that cell: held in the document when
/// the keeper asks the screen to finish, entered when the screen goes.
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

  /// The editing of the cell, open for as long as the screen shows it: nothing of
  /// it is in the document until the keeper asks the screen to finish.
  Editing? _editing;

  /// What entering the text ran into, if anything.
  Object? _refused;

  @override
  void initState() {
    super.initState();
    _attachment = widget.keeper.attach(
      onChange: () => setState(() {}),
      finish: () => _put(forGood: false),
    );
    _read();
  }

  /// A screen that goes enters what is typed on it, which ends its editing. It is
  /// one call, asked of the keeper before anyone can ask it to close: the editing
  /// is open already. (Two calls, one waited for before the other is made, would
  /// leave the second behind the close.)
  @override
  void dispose() {
    _put(forGood: true);
    _attachment.detach();
    _typing.dispose();
    super.dispose();
  }

  Future<void> _read() async {
    final keeper = widget.keeper;
    final cells = (await keeper.grid(keeper.root)).cells;
    if (!mounted || cells.isEmpty) return;
    if (_cell == null) _editing = await keeper.edit(cells.first.id);
    if (!mounted) return;
    setState(() {
      _cell = cells.first.id;
      _shown = cells.first.text;
    });
  }

  /// What is typed goes into the document: held there while the editing goes on,
  /// or entered `forGood`, which ends it. What the core says to it is shown, not
  /// thrown: nothing is unfinished then.
  Future<void> _put({required bool forGood}) async {
    final cell = _cell;
    final text = _typing.text;
    if (cell == null || text.isEmpty) return;
    try {
      var editing = _editing;
      // Over already when its document was let go of, or another editing opened.
      if (editing == null || !editing.open) {
        editing = _editing = await widget.keeper.edit(cell);
      }
      forGood ? await editing.enter(text) : await editing.hold(text);
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
