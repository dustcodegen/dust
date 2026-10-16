import 'package:dio/dio.dart' show InterceptorsWrapper;
import 'package:dust_dart/http.dart';
import 'package:primary_sdk_consumer/api.dart';
import 'package:primary_sdk_consumer/models.dart';
import 'package:test/test.dart';

void main() {
  late Dio dio;
  late PrimaryApi api;

  setUp(() {
    dio = Dio();
    api = PrimaryApi(dio);
  });

  tearDown(() => dio.close(force: true));

  test(
    'HTTP responses decode into primary models without a network request',
    () async {
      dio.interceptors.add(
        InterceptorsWrapper(
          onRequest: (options, handler) {
            expect(options.method, 'GET');
            expect(options.path, '/accounts/7');
            handler.resolve(
              Response<Object?>(
                requestOptions: options,
                data: <String, Object?>{'account_id': 7, 'name': 'Ada'},
              ),
            );
          },
        ),
      );
      expect(await api.account(7), const PrimaryAccount(id: 7, name: 'Ada'));
    },
  );

  test('HTTP bodies use the generated primary-model serializer', () async {
    const account = PrimaryAccount(id: 7, name: 'Ada', note: 'saved');
    dio.interceptors.add(
      InterceptorsWrapper(
        onRequest: (options, handler) {
          expect(options.method, 'POST');
          expect(options.path, '/accounts');
          expect(options.data, account.toJson());
          handler.resolve(
            Response<Object?>(requestOptions: options, data: options.data),
          );
        },
      ),
    );
    expect(await api.save(account), account);
  });
}
