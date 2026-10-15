import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'native_process.dart';

const int volumeLimit = 32 * 1024 * 1024;

/// Fail closed before writing: a path alone never authorizes filling a volume.
void validateBoundedVolume(
  Map<String, Object?> info, {
  required String mountPoint,
  required String device,
  required String uuid,
}) {
  final capacity = info['TotalSize'];
  if (uuid.isEmpty ||
      !RegExp(r'^/dev/disk\d+(s\d+)?$').hasMatch(device) ||
      info['MountPoint'] != mountPoint ||
      info['DeviceNode'] != device ||
      info['VolumeUUID'] != uuid ||
      info['FilesystemType'] != 'hfs' ||
      capacity is! int ||
      capacity < 8 * 1024 * 1024 ||
      capacity > volumeLimit) {
    throw StateError('Refusing non-owned/unbounded volume: $info');
  }
}

/// Only [create] can acquire a fresh, fixed-size, test-owned disk image.
final class BoundedVolume {
  BoundedVolume._(this.directory);
  final Directory directory;
  late final String image = '${directory.path}/volume.dmg';
  late final String mountPoint = '${directory.path}/mounted';
  late final String device;
  late final String uuid;
  late final int capacity;
  bool _closed = false;
  bool _unsettledMutation = false;

  static Future<BoundedVolume> create() async {
    if (!Platform.isMacOS) {
      throw UnsupportedError('The bounded disk-image gate requires macOS.');
    }
    final created = Directory.systemTemp.createTempSync('dust-enospc-');
    final volume = BoundedVolume._(
      Directory(created.resolveSymbolicLinksSync()),
    );
    try {
      final headroom = await nativeProcess('/bin/df', [
        '-Pk',
        volume.directory.path,
      ]);
      final fields = (headroom.stdout as String)
          .trim()
          .split('\n')
          .last
          .split(RegExp(r'\s+'));
      if (headroom.exitCode != 0 ||
          fields.length < 4 ||
          int.parse(fields[3]) < 512 * 1024) {
        throw StateError('At least 512 MiB host headroom is required.');
      }
      await volume._run('hdiutil', [
        'create',
        '-size',
        '32m',
        '-fs',
        'HFS+',
        '-layout',
        'NONE',
        '-type',
        'UDIF',
        '-nospotlight',
        '-volname',
        'DustSqliteFaults',
        volume.image,
      ]);
      if (File(volume.image).lengthSync() > volumeLimit + 1024 * 1024) {
        throw StateError('Disk image exceeded its fixed-size bound.');
      }
      await volume._run('hdiutil', [
        'attach',
        '-nobrowse',
        '-noautoopen',
        '-mountpoint',
        volume.mountPoint,
        volume.image,
      ]);
      final attachment = await volume._attachment();
      final entity = (attachment!['system-entities'] as List<Object?>)
          .cast<Map<String, Object?>>()
          .singleWhere((e) => e['mount-point'] == volume.mountPoint);
      volume.device = entity['dev-entry']! as String;
      final info = await volume._plist('diskutil', [
        'info',
        '-plist',
        volume.mountPoint,
      ]);
      volume.uuid = info['VolumeUUID']! as String;
      volume.capacity = info['TotalSize']! as int;
      await volume.verify();
      return volume;
    } catch (error) {
      try {
        await volume.close();
      } catch (cleanup) {
        throw StateError('Setup failed: $error; $cleanup');
      }
      rethrow;
    }
  }

  Future<void> verify() async {
    final attachment = await _attachment();
    final entities = attachment?['system-entities'] as List<Object?>?;
    if (entities == null ||
        !entities.cast<Map<String, Object?>>().any(
          (e) => e['dev-entry'] == device && e['mount-point'] == mountPoint,
        )) {
      throw StateError('Image/mount identity changed; refusing filler writes.');
    }
    validateBoundedVolume(
      await _plist('diskutil', ['info', '-plist', mountPoint]),
      mountPoint: mountPoint,
      device: device,
      uuid: uuid,
    );
  }

  Future<({int bytes, int errno})> fill() async {
    await verify();
    final filler = await File(
      '$mountPoint/filler.bin',
    ).open(mode: FileMode.write);
    final chunk = Uint8List(64 * 1024)..fillRange(0, 64 * 1024, 0x5a);
    var written = 0;
    int? errno;
    try {
      while (written < volumeLimit) {
        await filler.writeFrom(chunk);
        written += chunk.length;
      }
      await filler.flush();
    } on FileSystemException catch (error) {
      if (error.osError?.errorCode != 28) rethrow;
      errno = error.osError!.errorCode;
    } finally {
      try {
        await filler.close();
      } on FileSystemException catch (error) {
        if (error.osError?.errorCode != 28) rethrow;
        errno = error.osError!.errorCode;
      }
    }
    if (errno == null || written == 0) {
      throw StateError('Expected actual OS ENOSPC within the fixed write cap.');
    }
    return (bytes: written, errno: errno);
  }

  Future<void> restoreSpace() async {
    await verify();
    await File('$mountPoint/filler.bin').delete();
  }

  Future<void> close() async {
    if (_closed) return;
    final attachment = await _attachment();
    if (attachment != null) {
      final entities = (attachment['system-entities'] as List<Object?>)
          .cast<Map<String, Object?>>();
      final whole = entities
          .map((e) => e['dev-entry'])
          .whereType<String>()
          .firstWhere((e) => RegExp(r'^/dev/disk\d+$').hasMatch(e));
      try {
        await _run('hdiutil', ['detach', whole]);
      } catch (error) {
        throw StateError('Detach failed; image preserved at $image: $error');
      }
    }
    if (await _attachment() != null) {
      throw StateError('Image still attached; preserved at $image');
    }
    if (_unsettledMutation) {
      throw StateError(
        'Image setup failed with uncertain OS state; preserved at $image',
      );
    }
    await directory.delete(recursive: true);
    _closed = true;
  }

  Future<Map<String, Object?>?> _attachment() async {
    final info = await _plist('hdiutil', ['info', '-plist']);
    final images = (info['images'] as List<Object?>)
        .cast<Map<String, Object?>>();
    return images.where((entry) => entry['image-path'] == image).firstOrNull;
  }

  Future<Map<String, Object?>> _plist(String tool, List<String> args) async {
    final result = await _run(tool, args);
    final file = File('${directory.path}/metadata.plist')
      ..writeAsStringSync(result.stdout as String);
    final converted = await _run('/usr/bin/plutil', [
      '-convert',
      'json',
      '-o',
      '-',
      file.path,
    ]);
    return jsonDecode(converted.stdout as String) as Map<String, Object?>;
  }

  Future<ProcessResult> _run(String tool, List<String> args) async {
    try {
      final result = await nativeProcess(tool, args);
      if (result.exitCode != 0) {
        throw StateError('$tool failed: ${result.stdout}\n${result.stderr}');
      }
      return result;
    } catch (_) {
      // Failed create/attach may leave asynchronous OS work using the image.
      if (tool == 'hdiutil' &&
          (args.first == 'create' || args.first == 'attach')) {
        _unsettledMutation = true;
      }
      rethrow;
    }
  }
}
