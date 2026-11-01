"""Boundary-rank and malformed-source guards for mixed R8 bricks."""
import importlib.util
from pathlib import Path
import struct
import unittest
import numpy as np

p=Path(__file__).with_name('ranked.py');s=importlib.util.spec_from_file_location('ranked_test_codec',p)
r=importlib.util.module_from_spec(s);s.loader.exec_module(r)


class MixedCodec(unittest.TestCase):
    def test_rank_boundaries_and_unset(self):
        values=np.zeros(512,np.uint8)
        indices=[0,31,32,63,64,511]
        values[indices]=[11,22,33,44,55,66]
        block,ranked=r.encode_brick(values)
        self.assertTrue(ranked)
        for index in range(512):
            self.assertEqual(r.lookup_local(block,1,index),int(values[index]))
        for index in [-1,512]:
            with self.assertRaises(IndexError):r.lookup_local(block,1,index)

    def test_dense_and_empty_frames(self):
        dense=np.arange(512,dtype=np.uint16).astype(np.uint8)
        block,ranked=r.encode_brick(dense)
        self.assertFalse(ranked);self.assertEqual(block,dense.tobytes())
        for index in [0,31,32,63,64,511]:self.assertEqual(r.lookup_local(block,0,index),int(dense[index]))
        for shape in [8,16]:
            empty=np.zeros((shape,)*3,np.uint8);blob,record=r.pack_frame(empty)
            self.assertEqual(record['active_bricks'],0)
            np.testing.assert_array_equal(r.unpack_frame(blob,shape,record),empty)

    def test_mixed_frame_preserves_crossbrick_positions(self):
        a=np.zeros((16,)*3,np.uint8)
        a[0,0,0]=3;a[0,7,7]=99;a[8:16,8:16,8:16]=255
        blob,record=r.pack_frame(a)
        self.assertEqual(record['ranked_bricks'],1);self.assertEqual(record['dense_bricks'],1)
        np.testing.assert_array_equal(r.unpack_frame(blob,16,record),a)
        pointer=struct.unpack_from('<I',blob,record['pointer_table_offset'])[0]
        self.assertEqual(pointer&~1,record['brick_data_offset'])
        self.assertEqual(r.lookup_local(blob,pointer,0),3)
        self.assertEqual(r.lookup_local(blob,pointer,63),99)

    def test_malformed_prefix_pointer_count_and_masks(self):
        a=np.zeros((16,)*3,np.uint8);a[0,0,0]=9;a[0,0,1]=10
        blob,record=r.pack_frame(a)
        pointer=struct.unpack_from('<I',blob,record['pointer_table_offset'])[0]&~1
        broken=bytearray(blob);struct.pack_into('<H',broken,pointer+64+2,300)
        with self.assertRaisesRegex(ValueError,'prefix'):r.unpack_frame(broken,16,record)
        for replacement in [pointer+2,pointer+4,0,len(blob)+4]:
            broken=bytearray(blob);struct.pack_into('<I',broken,record['pointer_table_offset'],replacement|1)
            with self.assertRaises(ValueError):r.unpack_frame(broken,16,record)
        with self.assertRaises(ValueError):r.unpack_frame(blob,16,dict(record,active_bricks=2))
        broken=bytearray(blob);broken[pointer:pointer+64]=bytes(64)
        with self.assertRaises(ValueError):r.unpack_frame(broken,16,record)
        with self.assertRaises(ValueError):r.unpack_frame(blob[:-1],16,record)


if __name__=='__main__':unittest.main()
