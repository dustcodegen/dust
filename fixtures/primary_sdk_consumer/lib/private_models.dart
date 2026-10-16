import 'package:dust_dart/db.dart';
import 'package:dust_dart/serde.dart';

part 'private_models.g.dart';

/// A primary model retaining private storage and public call labels.
@Derive([Eq(), CopyWith(), Serialize(), Deserialize(), FromRow()])
class const PrivateAccount({
  @SerDe(rename: 'display_name')
  @Sqlx(rename: 'display_name')
  required final String _name,
  @SerDe(rename: 'note') @Sqlx(rename: 'note') final String? _note,
}) with _$PrivateAccount {
  /// Creates an account using public `name` and `note` arguments.
  this;

  /// Reads generated JSON using public constructor labels.
  factory PrivateAccount.fromJson(Map<String, Object?> json) =>
      _$PrivateAccountFromJson(json);

  /// The visible name, backed by private storage.
  String get name => _name;

  /// The optional note, backed by private storage.
  String? get note => _note;
}

/// A private nested field still has a public copyWith path.
@Derive([CopyWith()])
class const PrivateEnvelope({final PrivateAccount? _account})
    with _$PrivateEnvelope {
  /// Creates an envelope.
  this;

  /// The wrapped account.
  PrivateAccount? get account => _account;
}

/// Private positional fields must not collide with generated receiver locals.
@Derive([CopyWith()])
class const PrivateLocals(
  final String _self,
  final String _then,
  final String _value,
) with _$PrivateLocals {
  /// Creates the collision regression model.
  this;

  /// Values used to verify private field access separately from call labels.
  (String, String, String) get values => (_self, _then, _value);
}

/// Non-nullable private nested fields also expose the public copy path.
@Derive([CopyWith()])
class const PrivateOwner({required final PrivateAccount _account})
    with _$PrivateOwner {
  /// Creates an owner.
  this;

  /// The owned account.
  PrivateAccount get account => _account;
}

/// Traditional private initializing formals share the same call-label rule.
@Derive([CopyWith(), Serialize(), Deserialize()])
class TraditionalPrivate with _$TraditionalPrivate {
  /// Creates a traditional model using a public `name` argument.
  const TraditionalPrivate({required this._name});

  final String _name;

  /// The private field's visible value.
  String get name => _name;

  /// Reads the generated representation.
  factory TraditionalPrivate.fromJson(Map<String, Object?> json) =>
      _$TraditionalPrivateFromJson(json);
}
