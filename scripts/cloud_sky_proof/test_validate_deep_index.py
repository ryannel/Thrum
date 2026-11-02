"""Minimal synthetic admission regressions; never substitutes for actual GPU data."""
import json
from pathlib import Path
import tempfile
import unittest

import numpy as np
import validate_deep_index as ref


def fixture(root):
    nodes=np.zeros((4356,48,2),dtype='<f4');nodes[:,1,0]=1
    nodes[0,1,0]=.5;nodes[0,2,0]=1
    metadata=np.tile(np.array([2,1,.005,.005,512,512,1,0],dtype='<f4'),(4356,1))
    metadata[0,[0,5,6]]=[3,513,2]
    tags=np.full((4356,512),255,dtype=np.uint8);tags[0,:256]=0;tags[0,256:]=1
    paths=np.zeros((1,513,4),dtype='<f4');paths[0,:,0]=np.arange(513,dtype=np.float32)/512;paths[:,:,3]=1
    payloads=dict(nodes=nodes.tobytes(),metadata=metadata.tobytes(),index=tags.tobytes(),
                  field=np.zeros((3,3,3,4),dtype='<f2').tobytes(),paths=paths.tobytes())
    shapes=dict(nodes=[66,66,48,2],metadata=[66,66,8],index=[66,66,128],field=[3,3,3,4],paths=[1,513,4])
    words=np.zeros(48,dtype='<u4');words[40:43]=1;words[43]=64
    binaries={}
    for name,data in payloads.items():
        filename=name+'.bin';(root/filename).write_bytes(data)
        fmt='little-endian '+('uint32 packed4xu8' if name=='index' else 'float16' if name=='field' else 'float32')
        binaries[name]=dict(filename=filename,bytes=len(data),sha256=ref.sha(data),shape=shapes[name],format=fmt)
    receipt=dict(schema='thrum.deep-sun-probe.v1',generation_valid=True,logical_dimensions=[1,1,1],field_physical_dimensions=[3,3,3],
                 binaries=binaries,uniform_u32=words.tolist(),identity={'uniform_sha256':ref.sha(words.tobytes()),'synthetic_test':True},
                 selected_columns=[dict(physical_xy=[0,0],linear_index=0)],metadata_summary={'index_build':dict(rows=1,interval_visits=512,u32_word_stores=128,anchor_advances=1)},
                 query=dict(indexed_lookup_exercised=True,binary_reference_forced_locally=True,
                            rows=[dict(projected_depth_m=.5,terms=[1,1],binary_reference_terms=[1,1],valid=True,binary_reference_valid=True,
                                       selected_vs_binary_bitwise_equal=True,index_lookup_diagnostics=dict(valid=True,binary_retries=0,adjacent_corrections=0))],
                            index_lookup_diagnostics=dict(valid_points=1,binary_retries=0,adjacent_corrections=0),
                            counters=dict(unused_view_caps=0,fallback_requests=0,invalid_footprints=0,unsupported=0)))
    save(root,receipt)
    return receipt


def save(root,receipt):
    (root/'receipt.json').write_text(json.dumps(receipt))


def mutate_payload(root,receipt,name,change,rehash=True):
    entry=receipt['binaries'][name];path=root/entry['filename'];data=bytearray(path.read_bytes());change(data);path.write_bytes(data)
    if rehash:entry['sha256']=ref.sha(data)
    save(root,receipt)


class IndexAdmissionTests(unittest.TestCase):
    def setUp(self):
        self.directory=tempfile.TemporaryDirectory();self.root=Path(self.directory.name);self.receipt=fixture(self.root)
    def tearDown(self):
        self.directory.cleanup()
    def run_reference(self):
        return ref.validate_index(self.root,'receipt.json')
    def test_valid_defined_and_unspecified_rows(self):
        result=self.run_reference();self.assertTrue(result['finite_index_gate_pass'])
        self.assertEqual(result['decoded_defined_entries'],512);self.assertEqual(result['skipped_count2_rows'],4355)
        # All unspecified rows already contain255 and must never be inspected.
        self.assertEqual(result['defined_rows'],1)
    def test_sha_mutation_rejected_before_semantics(self):
        mutate_payload(self.root,self.receipt,'index',lambda b:b.__setitem__(0,1),False)
        with self.assertRaisesRegex(ValueError,'SHA/length'):self.run_reference()
    def test_layout_rejected(self):
        self.receipt['binaries']['index']['shape']=[66,66,127];save(self.root,self.receipt)
        with self.assertRaisesRegex(ValueError,'shape/size'):self.run_reference()
    def test_wrong_defined_class_rejected_after_rehash(self):
        mutate_payload(self.root,self.receipt,'index',lambda b:b.__setitem__(0,1))
        with self.assertRaisesRegex(ValueError,'interval class'):self.run_reference()
    def test_outside_logical_segment_rejected_after_rehash(self):
        mutate_payload(self.root,self.receipt,'index',lambda b:b.__setitem__(0,255))
        with self.assertRaisesRegex(ValueError,'logical segment'):self.run_reference()
    def test_unspecified_row_mutation_allowed(self):
        mutate_payload(self.root,self.receipt,'index',lambda b:b.__setitem__(512,17))
        self.assertTrue(self.run_reference()['finite_index_gate_pass'])
    def test_nonindexed_selector_does_not_gain_index_parity_pass(self):
        self.receipt['uniform_u32'][43]=0
        self.receipt['identity']['uniform_sha256']=ref.sha(np.array(self.receipt['uniform_u32'],dtype='<u4').tobytes())
        self.receipt['query']['indexed_lookup_exercised']=False;save(self.root,self.receipt)
        result=self.run_reference();self.assertTrue(result['capture_structurally_valid']);self.assertFalse(result['finite_index_gate_pass'])
    def test_gpu_parity_claim_checked_from_float32_bits(self):
        row=self.receipt['query']['rows'][0];row['terms'][0]=float(np.nextafter(np.float32(1),np.float32(0)));save(self.root,self.receipt)
        with self.assertRaisesRegex(ValueError,'decoded bits'):self.run_reference()
        row['selected_vs_binary_bitwise_equal']=False;save(self.root,self.receipt)
        self.assertFalse(self.run_reference()['finite_index_gate_pass'])
    def test_false_lookup_diagnostics_do_not_pass(self):
        self.receipt['query']['rows'][0]['index_lookup_diagnostics']['valid']=False;save(self.root,self.receipt)
        self.assertFalse(self.run_reference()['finite_index_gate_pass'])
    def test_admitted_node_count_rejects(self):
        def change(data):np.frombuffer(data,dtype='<f4')[0]=1
        mutate_payload(self.root,self.receipt,'metadata',change)
        with self.assertRaisesRegex(ValueError,'failure/count'):self.run_reference()


if __name__=='__main__':
    unittest.main()
