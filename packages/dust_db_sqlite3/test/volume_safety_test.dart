import 'package:test/test.dart';

import '../integration_test/support/bounded_volume.dart';

void main() {
  const info = <String, Object?>{
    'MountPoint': '/owned/mounted',
    'DeviceNode': '/dev/disk99',
    'VolumeUUID': 'test-volume-uuid',
    'FilesystemType': 'hfs',
    'TotalSize': volumeLimit,
  };
  void validate(Map<String, Object?> value) => validateBoundedVolume(
    value,
    mountPoint: '/owned/mounted',
    device: '/dev/disk99',
    uuid: 'test-volume-uuid',
  );
  test('accepts only the bounded owned volume identity', () => validate(info));
  test(
    'accepts the minimum bounded capacity',
    () => validate({...info, 'TotalSize': 8 * 1024 * 1024}),
  );
  for (final (key, invalid) in <(String, Object?)>[
    ('MountPoint', '/'),
    ('DeviceNode', '/dev/disk0'),
    ('VolumeUUID', 'other'),
    ('FilesystemType', 'apfs'),
    ('TotalSize', volumeLimit + 1),
    ('TotalSize', 0),
    ('TotalSize', '33554432'),
    ('TotalSize', null),
  ]) {
    test('rejects mismatched $key=$invalid', () {
      expect(() => validate({...info, key: invalid}), throwsStateError);
    });
  }
  test(
    'rejects missing metadata',
    () => expect(() => validate({}), throwsStateError),
  );
  test(
    'rejects an empty expected UUID',
    () => expect(
      () => validateBoundedVolume(
        info,
        mountPoint: '/owned/mounted',
        device: '/dev/disk99',
        uuid: '',
      ),
      throwsStateError,
    ),
  );
  test(
    'rejects a non-device target',
    () => expect(
      () => validateBoundedVolume(
        info,
        mountPoint: '/owned/mounted',
        device: '/',
        uuid: 'test-volume-uuid',
      ),
      throwsStateError,
    ),
  );
}
