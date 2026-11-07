import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../language.dart';
import '../../renderer.dart';
import 'preview_client.dart';

class LabScreen extends StatefulWidget {
  const LabScreen({super.key});

  @override
  State<LabScreen> createState() => _LabScreenState();
}

class _LabScreenState extends State<LabScreen> {
  final _client = PreviewClient();
  final _source = TextEditingController();
  final _editorFocus = FocusNode();
  final _editorKey = GlobalKey();
  final _previewKey = GlobalKey();
  Timer? _debounce;
  UiNode? _document;
  UiSyntaxException? _diagnostic;
  String? _loadError;
  String _loadedSource = '';
  bool _loading = false;
  bool _showPreview = false;
  int _revision = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_reload());
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _client.close();
    _source.dispose();
    _editorFocus.dispose();
    super.dispose();
  }

  Future<void> _reload() async {
    _debounce?.cancel();
    setState(() {
      _loading = true;
      _loadError = null;
    });
    try {
      final source = await _client.loadSource();
      if (!mounted) return;
      _source.text = source;
      _loadedSource = source;
      _parse();
    } on Object {
      if (!mounted) return;
      setState(() => _loadError = 'Could not load the source file.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  void _parse() {
    try {
      final document = parseUi(_source.text);
      setState(() {
        _diagnostic = null;
        _document = document;
        _revision++;
      });
    } on UiSyntaxException catch (error) {
      setState(() => _diagnostic = error);
    }
  }

  void _changed(String _) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 180), _parse);
    setState(() {});
  }

  void _goToError(UiSyntaxException error) {
    final lines = _source.text.split('\n');
    final row = (error.line - 1).clamp(0, lines.length - 1);
    final start = lines
        .take(row)
        .fold(0, (offset, line) => offset + line.length + 1);
    final offset = start + (error.column - 1).clamp(0, lines[row].length);
    _editorFocus.requestFocus();
    _source.selection = TextSelection.collapsed(offset: offset);
  }

  Future<void> _copy() async {
    await Clipboard.setData(ClipboardData(text: _source.text));
    if (mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('Copied UI source')));
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(
      title: const Text('Dust UI Lab'),
      actions: [
        IconButton(
          onPressed: _loading ? null : _reload,
          icon: const Icon(Icons.refresh),
          tooltip: 'Reload source from disk',
        ),
        IconButton(
          onPressed: _source.text.isEmpty ? null : _copy,
          icon: const Icon(Icons.copy_outlined),
          tooltip: 'Copy source',
        ),
      ],
    ),
    body: SafeArea(
      child: LayoutBuilder(
        builder: (context, constraints) {
          final editor = _editor(context);
          final preview = _preview(context);
          if (constraints.maxWidth >= 980) {
            return Row(
              children: [
                Expanded(child: editor),
                const VerticalDivider(width: 1),
                Expanded(child: preview),
              ],
            );
          }
          return Column(
            children: [
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: SegmentedButton<bool>(
                  segments: const [
                    ButtonSegment(value: false, label: Text('Source')),
                    ButtonSegment(value: true, label: Text('Preview')),
                  ],
                  selected: {_showPreview},
                  onSelectionChanged: (value) =>
                      setState(() => _showPreview = value.single),
                ),
              ),
              Expanded(
                child: IndexedStack(
                  index: _showPreview ? 1 : 0,
                  sizing: StackFit.expand,
                  children: [editor, preview],
                ),
              ),
            ],
          );
        },
      ),
    ),
  );

  Widget _editor(BuildContext context) {
    final error = _diagnostic;
    final colors = Theme.of(context).colorScheme;
    final dirty = _source.text != _loadedSource;
    return Padding(
      key: _editorKey,
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PanelTitle('Source', detail: dirty ? 'Browser edits' : '.dustui'),
          const SizedBox(height: 8),
          Expanded(
            child: TextField(
              controller: _source,
              focusNode: _editorFocus,
              readOnly: _loading,
              onChanged: _changed,
              expands: true,
              minLines: null,
              maxLines: null,
              autocorrect: false,
              enableSuggestions: false,
              textAlignVertical: TextAlignVertical.top,
              style: const TextStyle(fontFamily: 'monospace', fontSize: 14),
              decoration: InputDecoration(
                labelText: 'UI document source',
                alignLabelWithHint: true,
                filled: true,
                fillColor: colors.surfaceContainerLow,
                border: const OutlineInputBorder(),
              ),
            ),
          ),
          if (_loading) const LinearProgressIndicator(),
          if (_loadError case final message?) ...[
            const SizedBox(height: 8),
            Text(message, style: TextStyle(color: colors.error)),
          ],
          if (error != null) ...[
            const SizedBox(height: 8),
            Semantics(
              liveRegion: true,
              child: TextButton(
                onPressed: () => _goToError(error),
                style: TextButton.styleFrom(
                  foregroundColor: colors.error,
                  alignment: Alignment.centerLeft,
                ),
                child: Text('${error.line}:${error.column}  ${error.message}'),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _preview(BuildContext context) {
    final document = _document;
    return Padding(
      key: _previewKey,
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PanelTitle(
            'Preview',
            detail: _diagnostic != null && document != null
                ? 'Last valid source'
                : 'Interactive',
          ),
          const SizedBox(height: 8),
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                border: Border.all(color: Theme.of(context).dividerColor),
                borderRadius: BorderRadius.circular(12),
              ),
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(12),
                child: Align(
                  alignment: Alignment.topCenter,
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 460),
                    child: document == null
                        ? Padding(
                            padding: const EdgeInsets.all(16),
                            child: Text(
                              _loading
                                  ? 'Loading source…'
                                  : 'Enter a valid UI document.',
                            ),
                          )
                        : DustUiView(
                            key: ValueKey(_revision),
                            document: document,
                            request: _client.request,
                          ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _PanelTitle extends StatelessWidget {
  const _PanelTitle(this.title, {required this.detail});

  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      Text(title, style: Theme.of(context).textTheme.titleMedium),
      const Spacer(),
      Text(detail, style: Theme.of(context).textTheme.labelMedium),
    ],
  );
}
