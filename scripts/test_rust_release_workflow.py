"""Keep manual Rust checks separate from release publication."""

import unittest
from pathlib import Path

WORKFLOW = Path(__file__).resolve().parent.parent / ".github/workflows/rust-release.yml"


class RustReleaseWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.workflow = WORKFLOW.read_text()

    def test_only_manual_dispatch_and_check_only_default(self):
        triggers = self.workflow.split("\non:\n", 1)[1].split("\npermissions:\n", 1)[0]
        self.assertEqual(triggers, """  workflow_dispatch:
    inputs:
      publish:
        description: Build and publish the pinned original v0.2.1 release (otherwise check selected ref only)
        type: boolean
        default: false
""")
        self.assertIn("\npermissions:\n  contents: read\n", self.workflow)

    def test_check_only_uses_exact_dispatch_commit(self):
        checks = self.workflow.split("  rust-checks:\n", 1)[1].split("  build-binaries:\n", 1)[0]
        self.assertIn("ref: ${{ inputs.publish && env.RELEASE_TAG || github.sha }}", checks)
        self.assertIn("name: Verify exact original release source\n        if: inputs.publish", checks)
        self.assertIn("CHECK_COMMIT: ${{ github.sha }}", checks)
        self.assertIn('test "$(git rev-parse HEAD)" = "$CHECK_COMMIT"', checks)
        self.assertNotIn("contents: write", checks)

    def test_build_and_publish_require_explicit_publish(self):
        self.assertIn("  build-binaries:\n    if: inputs.publish\n", self.workflow)
        self.assertIn("  publish-release:\n    if: inputs.publish\n", self.workflow)
        build = self.workflow.split("  build-binaries:\n", 1)[1].split("  publish-release:\n", 1)[0]
        self.assertIn("ref: ${{ env.RELEASE_TAG }}", build)
        self.assertIn('test "$(git rev-parse HEAD)" = "$SOURCE_COMMIT"', build)
        self.assertIn('test "$(gh release view "$RELEASE_TAG" --json isDraft --jq .isDraft)" = true', self.workflow)


if __name__ == "__main__":
    unittest.main()
