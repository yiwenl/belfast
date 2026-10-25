import struct
import tempfile
import unittest
from pathlib import Path

from remove_courtyard_tree import remove_tree, tree_mask


def fixture(points):
    header = ("ply\nformat binary_little_endian 1.0\n"
              f"element vertex {len(points)}\n"
              "property float z\nproperty float opacity\nproperty float x\nproperty float y\n"
              "end_header\n").encode()
    records = [struct.pack("<ffff", z, i + 0.25, x, y) for i, (x, y, z) in enumerate(points)]
    return header, records


class TreeRemovalTests(unittest.TestCase):
    def test_spatial_mask_preserves_ground_and_background(self):
        self.assertTrue(tree_mask(0.1, -1, 5.9))
        self.assertTrue(tree_mask(0.05, 0.78, 5.98))
        self.assertFalse(tree_mask(0.05, 0.85, 5.98))
        self.assertFalse(tree_mask(1, 0.78, 5.98))
        self.assertFalse(tree_mask(0.1, -1, 12))
        self.assertFalse(tree_mask(-2.5, -1, 5.9))

    def test_keeps_record_bytes_and_original_file_unchanged(self):
        points = [(0.1, -1, 5.9), (0.1, 0.85, 5.9), (0.1, -1, 12)]
        header, records = fixture(points)
        original = header + b"".join(records)
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory) / "source.ply", Path(directory) / "result.ply"
            source.write_bytes(original)
            report = remove_tree(source, output)
            self.assertEqual(report["removed_records"], 1)
            self.assertEqual(source.read_bytes(), original)
            self.assertEqual(output.read_bytes(), header.replace(b"vertex 3", b"vertex 2") + b"".join(records[1:]))
            with self.assertRaises(FileExistsError):
                remove_tree(source, output)
            with self.assertRaises(ValueError):
                remove_tree(source, source, overwrite=True)

    def test_truncated_input_does_not_create_output(self):
        header, records = fixture([(0, 0, 0)])
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory) / "source.ply", Path(directory) / "result.ply"
            source.write_bytes(header + records[0][:-1])
            with self.assertRaises(ValueError):
                remove_tree(source, output)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
