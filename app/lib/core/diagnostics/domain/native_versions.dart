import 'package:flutter/foundation.dart';

/// Versions of the native Rust crates, as a plain Dart model.
@immutable
class NativeVersions {
  const NativeVersions({
    required this.domain,
    required this.engine,
    required this.infra,
  });

  final String domain;
  final String engine;
  final String infra;

  @override
  bool operator ==(Object other) =>
      other is NativeVersions &&
      other.domain == domain &&
      other.engine == engine &&
      other.infra == infra;

  @override
  int get hashCode => Object.hash(domain, engine, infra);
}
