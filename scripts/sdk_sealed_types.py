"""Real compiler-negative contracts for typed sealed dispatch and generated variants."""

from pathlib import Path


def check_sealed_types(run, dart: Path, app: Path, output: Path, env: dict[str, str]) -> list[str]:
    """Require only the intended compiler error and restore each staged source."""
    cases = {
        "result": ("declared", "Event<int, List<int>> value = _$EventFromJson<num, List<num>>({'type': 'empty'});", ("can't be assigned", "Event<num", "Event<int")),
        "receiver": ("declared", "const $EventSerializer<int, List<int>>().serialize(const Done<List<num>, num>.create('wrong'));", ("can't be assigned", "Done<List<num>", "Event<int")),
        "bound": ("declared", "_$EventFromJson<int, List<String>>({'type': 'empty'});", ("doesn't conform", "List<String>", "List<A>")),
        "selected": ("declared", "Done<List<String>, String> value = _dustEventDoneDeserialize<List<int>, int>({'label': 'wrong'});", ("can't be assigned", "Done<List<int>", "Done<List<String>")),
        "generated-bound": ("generated", "GeneratedDone<List<String>, int>? value = null;", ("doesn't conform", "List<String>", "List<A>")),
        "generated-result": ("generated", "GeneratedDone<List<String>, String> value = GeneratedDone<List<int>, int>.fromJson({'label': 'wrong'});", ("can't be assigned", "GeneratedDone<List<int>", "GeneratedDone<List<String>")),
        "codec": ("generated", "GeneratedAt<List<String>, String> value = const $GeneratedAtDeserializer<List<int>, int>().deserialize({'date': 1});", ("can't be assigned", "GeneratedAt<List<int>", "GeneratedAt<List<String>")),
        "nullable": ("generated", "GeneratedCount<int, List<int>>(count: 'wrong');", ("can't be assigned", "String", "int?")),
        "adjacent": ("modes", "Envelope<String> value = const $EnvelopeDeserializer<int>().deserialize({'kind': 'stamped', 'data': {'at': 1}});", ("can't be assigned", "Envelope<int>", "Envelope<String>")),
        "untagged": ("modes", "Choice<String> value = const $ChoiceDeserializer<int>().deserialize({'number': 1});", ("can't be assigned", "Choice<int>", "Choice<String>")),
        "json-key": ("scope", "CapturedDone.fromJson(<int, Object?>{1: 3});", ("can't be assigned", "Map<int", "Map<String")),
    }
    for name, (kind, body, markers) in cases.items():
        source = app / f"lib/sealed_{kind}.dart"
        original = source.read_bytes()
        try:
            source.write_bytes(original + f"\nvoid main() {{ {body} }}\n".encode())
            result = run([str(dart), "compile", "kernel", str(source), "-o", str(output / f"{name}.dill")], app, expected=254, env=env)
            message = result.stdout + result.stderr
            if result.returncode != 254 or "Error:" not in message or not all(marker in message for marker in markers):
                raise ValueError(f"Missing expected sealed {name} type error")
        finally:
            source.write_bytes(original)
    return list(cases)
