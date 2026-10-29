// GENERATED CODE - DO NOT MODIFY BY HAND.

import 'package:dio/dio.dart';

/// Dio client generated from Dust server routes.
final class HealthRoutesClient {
  /// Creates a client using [Dio] and an optional base URL override.
  HealthRoutesClient(this._dio, {String? baseUrl}) : _baseUrl = baseUrl;

  final Dio _dio;
  final String? _baseUrl;

  /// Calls `GET /health/`.
  Future<String> health() async {
    final _queryParameters = <String, dynamic>{};
    final _headers = <String, dynamic>{};
    final _extra = <String, dynamic>{};
    final Object? _data = null;
    final _options = Options(
      method: 'GET',
      headers: _headers,
      extra: _extra,
      contentType: null,
    );
    final _result = await _dio.fetch<String>(
      _setStreamType<String>(
        _options
            .compose(
              _dio.options,
              '/health/',
              queryParameters: _queryParameters,
              data: _data,
              cancelToken: null,
              onSendProgress: null,
              onReceiveProgress: null,
            )
            .copyWith(
              baseUrl: _combineBaseUrls(
                _dio.options.baseUrl,
                _baseUrl,
              ),
            ),
      ),
    );
    return _result.data as String;
  }
}

RequestOptions _setStreamType<T>(RequestOptions requestOptions) {
  if (T != dynamic &&
      requestOptions.responseType != ResponseType.bytes &&
      requestOptions.responseType != ResponseType.stream) {
    if (T == ResponseBody) {
      requestOptions.responseType = ResponseType.stream;
    } else if (T == String) {
      requestOptions.responseType = ResponseType.plain;
    } else {
      requestOptions.responseType = ResponseType.json;
    }
  }
  return requestOptions;
}

String _combineBaseUrls(String dioBaseUrl, String? baseUrl) {
  if (baseUrl == null || baseUrl.trim().isEmpty) {
    return dioBaseUrl;
  }
  final url = Uri.parse(baseUrl);
  if (url.isAbsolute) {
    return url.toString();
  }
  return Uri.parse(dioBaseUrl).resolveUri(url).toString();
}

Response<T> _buildResponse<T>(Response<dynamic> response, T data) {
  return Response<T>(
    data: data,
    headers: response.headers,
    isRedirect: response.isRedirect,
    redirects: response.redirects,
    requestOptions: response.requestOptions,
    statusCode: response.statusCode,
    statusMessage: response.statusMessage,
    extra: response.extra,
  );
}
