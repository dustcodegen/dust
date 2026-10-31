import 'dart:ffi';

typedef IoUringRequestNative = Void Function(
  Uint64,
  Pointer<Uint8>,
  Uint64,
);

final class IoUringProbeBindings {
  IoUringProbeBindings(DynamicLibrary library)
      : start = library.lookupFunction<
            Pointer<Void> Function(
              Pointer<NativeFunction<IoUringRequestNative>>,
              Uint16,
            ),
            Pointer<Void> Function(
              Pointer<NativeFunction<IoUringRequestNative>>,
              int,
            )>('dust_io_uring_server_start'),
        port = library.lookupFunction<Uint16 Function(Pointer<Void>),
            int Function(Pointer<Void>)>('dust_io_uring_server_port'),
        allocate = library.lookupFunction<Pointer<Uint8> Function(IntPtr),
            Pointer<Uint8> Function(int)>(
          'dust_io_uring_response_buffer_new',
        ),
        respond = library.lookupFunction<
            Bool Function(
              Pointer<Void>,
              Uint64,
              Uint16,
              Pointer<Uint8>,
              IntPtr,
            ),
            bool Function(
              Pointer<Void>,
              int,
              int,
              Pointer<Uint8>,
              int,
            )>('dust_io_uring_server_respond'),
        free = library.lookupFunction<Void Function(Pointer<Void>),
            void Function(Pointer<Void>)>('dust_io_uring_server_free');

  final Pointer<Void> Function(
    Pointer<NativeFunction<IoUringRequestNative>>,
    int,
  ) start;
  final int Function(Pointer<Void>) port;
  final Pointer<Uint8> Function(int) allocate;
  final bool Function(
    Pointer<Void>,
    int,
    int,
    Pointer<Uint8>,
    int,
  ) respond;
  final void Function(Pointer<Void>) free;
}
