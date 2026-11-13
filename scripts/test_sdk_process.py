"""SDK row environments cannot borrow another row's Flutter installation."""

import os
import unittest
from pathlib import Path
from unittest.mock import patch

from sdk_process import sdk_environment


class SdkEnvironmentTests(unittest.TestCase):
    """No SDK installation or compiler is needed for environment isolation."""

    def test_standalone_dart_clears_an_inherited_flutter_root(self) -> None:
        inherited = {"PATH": "/old/bin", "FLUTTER_ROOT": "/old/flutter"}
        with patch.dict(os.environ, inherited, clear=True):
            env = sdk_environment(Path("/dart-sdk/bin/dart"))
            self.assertNotIn("FLUTTER_ROOT", env)
            self.assertEqual(env["PATH"], str(Path("/dart-sdk/bin")) + os.pathsep + "/old/bin")
            self.assertEqual(dict(os.environ), inherited)

    def test_flutter_dart_uses_the_explicit_matching_row(self) -> None:
        inherited = {"PATH": "/old/bin", "FLUTTER_ROOT": "/wrong/flutter"}
        with patch.dict(os.environ, inherited, clear=True):
            env = sdk_environment(
                Path("/row/flutter/bin/cache/dart-sdk/bin/dart"), Path("/row/flutter")
            )
            self.assertEqual(env["FLUTTER_ROOT"], str(Path("/row/flutter")))
            self.assertEqual(
                env["PATH"], str(Path("/row/flutter/bin/cache/dart-sdk/bin")) + os.pathsep + "/old/bin"
            )
            self.assertEqual(dict(os.environ), inherited)

    def test_missing_path_does_not_add_the_current_directory(self) -> None:
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(sdk_environment(Path("/sdk/bin/dart")), {"PATH": str(Path("/sdk/bin"))})


if __name__ == "__main__":
    unittest.main()
