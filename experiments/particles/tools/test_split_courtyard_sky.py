import struct
import tempfile
import unittest
from pathlib import Path

from split_courtyard_sky import split_sky, sky_edge_mask


class SplitSkyTest(unittest.TestCase):
    def test_edge_cleanup_preserves_ground_and_pale_facades(self):
        bright = [.85, .88, .95]
        scales = [-2, -3, -8]
        self.assertTrue(sky_edge_mask(5, -4, 20, bright, scales))
        self.assertFalse(sky_edge_mask(5, .8, 20, bright, scales))
        self.assertFalse(sky_edge_mask(0, -3, 18, bright, scales))
        self.assertFalse(sky_edge_mask(0, -1.5, -2, bright, scales))
        self.assertFalse(sky_edge_mask(5, -4, 20, [.2, .2, .2], scales))

    def test_partition_preserves_all_record_bytes_and_property_order(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Deliberately reordered position properties, and an unrelated attribute.
            rows = [struct.pack('<4f', 18, -5.8, 0, 42),
                    struct.pack('<4f', 50, -40, 10, -7),
                    struct.pack('<4f', 0, .8, 3, 99)]
            header = b'ply\nformat binary_little_endian 1.0\nelement vertex 3\nproperty float z\nproperty float y\nproperty float x\nproperty float extra\nend_header\n'
            source = root / 'source.ply'
            source.write_bytes(header + b''.join(rows))
            report = split_sky(source, root/'foreground.ply', root/'sky.ply')
            self.assertEqual(report['sky_records'], 1)
            self.assertEqual((root/'foreground.ply').read_bytes().split(b'end_header\n')[1], rows[0]+rows[2])
            self.assertEqual((root/'sky.ply').read_bytes().split(b'end_header\n')[1], rows[1])
            self.assertEqual(source.read_bytes(), header + b''.join(rows))
            with self.assertRaises(FileExistsError):
                split_sky(source, root/'foreground.ply', root/'sky.ply')

    def test_refuses_aliased_output_before_touching_source(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)/'source.ply'
            source.write_bytes(b'original')
            with self.assertRaises(ValueError):
                split_sky(source, source, Path(directory)/'sky.ply', True)
            self.assertEqual(source.read_bytes(), b'original')

    def test_truncated_payload_creates_no_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'source.ply'
            source.write_bytes(b'ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n')
            with self.assertRaises(ValueError):
                split_sky(source, root/'fg.ply', root/'sky.ply')
            self.assertFalse((root/'fg.ply').exists())
            self.assertFalse((root/'sky.ply').exists())


if __name__ == '__main__':
    unittest.main()
