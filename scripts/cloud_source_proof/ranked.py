"""Lossless mixed dense/ranked R8 brick codec for the direct authored proof.

No source/cadence/precision change. Pointer lowbit selects ranked encoding;
other pointer bits are a four-byte-aligned offset relative to the frame start.
"""
from __future__ import annotations

import argparse
from collections import OrderedDict
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import struct
import time

import numpy as np

ENCODING = 'thrum.cloud-source-proof.mixed-bricks.v1'
SCHEMA = 'thrum.cloud-source-proof.direct.v1'
EMPTY = 65535


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def aligned4(value):
    return (value+3)//4*4


def encode_brick(values):
    values=np.asarray(values,np.uint8).reshape(512)
    active=values!=0
    count=int(active.sum())
    if not count:raise ValueError('empty brick belongs in empty indirection')
    if 80+aligned4(count)>=512:
        return values.tobytes(),False
    mask_bytes=np.packbits(active,bitorder='little').tobytes()
    masks=struct.unpack('<16I',mask_bytes)
    prefix=[]
    running=0
    for group in range(8):
        prefix.append(running)
        running+=masks[2*group].bit_count()+masks[2*group+1].bit_count()
    data=mask_bytes+struct.pack('<8H',*prefix)+values[active].tobytes()
    return data+bytes(aligned4(len(data))-len(data)),True


def ranked_header(payload,offset):
    if offset<0 or offset+80>len(payload):raise ValueError('ranked header outside frame')
    masks=struct.unpack_from('<16I',payload,offset)
    prefix=struct.unpack_from('<8H',payload,offset+64)
    count=0
    for group in range(8):
        if prefix[group]!=count:raise ValueError('malformed rank prefix')
        count+=masks[2*group].bit_count()+masks[2*group+1].bit_count()
    size=80+aligned4(count)
    if count==0 or size>=512:raise ValueError('invalid ranked nonzero count/mode')
    if offset+size>len(payload):raise ValueError('ranked values outside frame')
    values=payload[offset+80:offset+80+count]
    if 0 in values:raise ValueError('ranked nonzero values include zero')
    if any(payload[offset+80+count:offset+size]):raise ValueError('ranked padding is not zero')
    return masks,prefix,count,size


def lookup_local(payload,tagged_pointer,index):
    if not 0<=index<512:raise IndexError(index)
    offset=tagged_pointer & ~1
    if offset%4:raise ValueError('brick pointer alignment')
    if not tagged_pointer&1:
        if offset+512>len(payload):raise ValueError('dense brick outside frame')
        return payload[offset+index]
    masks,prefix,count,size=ranked_header(payload,offset)
    word=index//32
    bit=index%32
    if not (masks[word]>>bit)&1:return 0
    rank=(prefix[index//64]+(masks[word-1].bit_count() if word%2 else 0)+
          (masks[word]&((1<<bit)-1)).bit_count())
    if rank>=count:raise ValueError('rank outside nonzero values')
    return payload[offset+80+rank]


def pack_frame(codes):
    codes=np.asarray(codes)
    side=codes.shape[0]
    if codes.dtype!=np.uint8 or codes.shape!=(side,)*3 or side%8:
        raise ValueError('expected cubic R8 codes, side divisible by8')
    n=side//8
    bricks=np.ascontiguousarray(codes.reshape(n,8,n,8,n,8).transpose(0,2,4,1,3,5)).reshape(n**3,512)
    active=np.any(bricks!=0,axis=1)
    count=int(active.sum())
    if count>=EMPTY:raise ValueError('u16 brick index overflow')
    indices=np.full(n**3,EMPTY,'<u2')
    indices[active]=np.arange(count,dtype=np.uint16)
    occupancy=np.packbits(active,bitorder='little').tobytes()
    index_bytes=indices.tobytes()
    table_start=len(index_bytes)+len(occupancy)
    body_start=aligned4(table_start+4*count)
    payload=bytearray(index_bytes+occupancy+bytes(body_start-table_start))
    ranked_count=0
    for index,brick in enumerate(bricks[active]):
        block,ranked=encode_brick(brick)
        offset=len(payload)
        assert offset%4==0
        struct.pack_into('<I',payload,table_start+4*index,offset|int(ranked))
        payload.extend(block)
        ranked_count+=int(ranked)
    return bytes(payload),{'index_bytes':len(index_bytes),'occupancy_bytes':len(occupancy),
        'active_bricks':count,'pointer_table_offset':table_start,'pointer_table_bytes':4*count,
        'brick_data_offset':body_start,'ranked_bricks':ranked_count,'dense_bricks':count-ranked_count,
        'bytes':len(payload)}


def unpack_frame(payload,side,record):
    n=side//8
    expected_index=n**3*2
    expected_occupancy=(n**3+7)//8
    count=int(record['active_bricks'])
    if record['index_bytes']!=expected_index or record['occupancy_bytes']!=expected_occupancy:
        raise ValueError('frame header dimensions disagree')
    table_start=expected_index+expected_occupancy
    body_start=aligned4(table_start+4*count)
    if count<0 or count>=EMPTY or len(payload)<body_start:
        raise ValueError('invalid active count/pointer table length')
    if record.get('pointer_table_offset')!=table_start or record.get('pointer_table_bytes')!=4*count or record.get('brick_data_offset')!=body_start:
        raise ValueError('pointer table metadata disagreement')
    indices=np.frombuffer(payload,dtype='<u2',count=n**3)
    occupied=np.unpackbits(np.frombuffer(payload,dtype=np.uint8,count=expected_occupancy,offset=expected_index),bitorder='little')
    if np.any(occupied[n**3:]):raise ValueError('nonzero occupancy padding')
    valid=indices!=EMPTY
    if not np.array_equal(occupied[:n**3].astype(bool),valid):raise ValueError('index/occupancy mismatch')
    if int(valid.sum())!=count or not np.array_equal(np.sort(indices[valid]),np.arange(count,dtype=np.uint16)):
        raise ValueError('active count/indirection disagreement')
    if any(payload[table_start+4*count:body_start]):raise ValueError('nonzero header alignment padding')
    bricks=np.zeros((n**3,512),np.uint8)
    decoded=[]
    cursor=body_start
    for active_index in range(count):
        tagged=struct.unpack_from('<I',payload,table_start+4*active_index)[0]
        offset=tagged & ~1
        if offset%4 or offset!=cursor:
            raise ValueError('brick pointer misaligned/noncanonical/overlapping')
        if tagged&1:
            masks,prefix,nonzero,size=ranked_header(payload,offset)
            active=np.unpackbits(np.frombuffer(payload,dtype=np.uint8,count=64,offset=offset),bitorder='little').astype(bool)
            brick=np.zeros(512,np.uint8)
            brick[active]=np.frombuffer(payload,dtype=np.uint8,count=nonzero,offset=offset+80)
        else:
            size=512
            if offset+size>len(payload):raise ValueError('dense brick outside frame')
            brick=np.frombuffer(payload,dtype=np.uint8,count=512,offset=offset).copy()
            if not np.any(brick):raise ValueError('active dense brick is empty')
        decoded.append(brick)
        cursor=offset+size
    if cursor!=len(payload) or record['bytes']!=len(payload):raise ValueError('frame byte/count disagreement')
    if count:
        decoded=np.asarray(decoded,np.uint8)
        bricks[valid]=decoded[indices[valid]]
    return bricks.reshape(n,n,n,8,8,8).transpose(0,3,1,4,2,5).reshape((side,)*3)


class Frames:
    def __init__(self,metadata,payload):
        self.metadata=metadata;self.payload=payload;self.cache=OrderedDict()
        side=metadata['density_side'];self.shape=(len(metadata['density_frames']),side,side,side)
        self.dtype=np.dtype(np.float32);self.ndim=4
    def __len__(self):return self.shape[0]
    def frame(self,index):
        index=int(index)
        if index<0:index+=len(self)
        if not 0<=index<len(self):raise IndexError(index)
        record=self.metadata['density_frames'][index];key=record['chunk_id']
        if key not in self.cache:
            block=self.payload[record['offset']:record['offset']+record['bytes']]
            if hashlib.sha256(block).hexdigest()!=record['sha256']:raise ValueError('mixed frame hash mismatch')
            code=unpack_frame(block,self.shape[1],record)
            if hashlib.sha256(code.tobytes()).hexdigest()!=record['quantized_frame_sha256']:
                raise ValueError('mixed decoded code hash mismatch')
            field=code.astype(np.float32)*np.float32(self.metadata['runtime_sigma_max_per_m']/255)
            field.setflags(write=False);self.cache[key]=field
            while len(self.cache)>2:self.cache.popitem(last=False)
        self.cache.move_to_end(key);return self.cache[key]
    def __getitem__(self,key):
        if isinstance(key,tuple):
            index,rest=key[0],key[1:];selected=self[index]
            return selected[(slice(None),)+rest] if isinstance(index,slice) else selected[rest]
        if isinstance(key,slice):
            indices=range(*key.indices(len(self)))
            return np.stack([self.frame(i) for i in indices]) if len(indices) else np.empty((0,)+self.shape[1:],np.float32)
        return self.frame(key)


def read_product(path):
    path=Path(path);metadata=json.loads((path/'cloudclip.json').read_text())
    if metadata['schema']!=SCHEMA or metadata.get('density_encoding_version')!=ENCODING:
        raise ValueError('unsupported mixed-brick product')
    payload=(path/'density.bin').read_bytes();record=metadata['products']['density.bin']
    if len(payload)!=record['bytes'] or hashlib.sha256(payload).hexdigest()!=record['sha256']:
        raise ValueError('mixed source product hash/length mismatch')
    frames=Frames(metadata,payload)
    return {'frames':frames,'density':frames,'read_frame':frames.frame,
            'times':np.asarray(metadata['source_times_s'],np.float64),
            'flow':np.empty((0,0,0,0,3),np.float32),'manifest':metadata}


def old_codec():
    path=Path(__file__).with_name('cook.py');spec=importlib.util.spec_from_file_location('ranked_input_codec',path)
    result=importlib.util.module_from_spec(spec);spec.loader.exec_module(result);return result


def repack(source,out):
    if out.exists():raise ValueError('output must be a new directory')
    begin=time.perf_counter();codec=old_codec()
    old=json.loads((source/'cloudclip.json').read_text());blob=(source/'density.bin').read_bytes()
    if hashlib.sha256(blob).hexdigest()!=old['products']['density.bin']['sha256']:
        raise ValueError('input source hash mismatch')
    frames=[];seen={};payload=bytearray();ranked_count=dense_count=0
    for index,record in enumerate(old['density_frames']):
        key=(record['offset'],record['bytes'])
        if key not in seen:
            part=blob[record['offset']:record['offset']+record['bytes']]
            if hashlib.sha256(part).hexdigest()!=record['sha256']:raise ValueError('input frame hash mismatch')
            codes=codec.unpack_density(part,old['density_side'],record)
            code_hash=hashlib.sha256(codes.tobytes()).hexdigest()
            if code_hash!=record['quantized_frame_sha256']:raise ValueError('input code hash mismatch')
            block,new=pack_frame(codes)
            if not np.array_equal(unpack_frame(block,old['density_side'],new),codes):
                raise ValueError('mixed codec is not bit-exact')
            new.update(offset=len(payload),sha256=hashlib.sha256(block).hexdigest(),
                       quantized_frame_sha256=code_hash,chunk_id=len(seen))
            seen[key]=new;payload.extend(block)
            ranked_count+=new['ranked_bricks'];dense_count+=new['dense_bricks']
        frames.append(dict(seen[key],source_frame=record['source_frame']))
    out.mkdir(parents=True)
    (out/'density.bin').write_bytes(payload)
    receipt=old['evidence_products']['diagnostics.json']
    if sha(source/'diagnostics.json')!=receipt['sha256']:raise ValueError('original optical receipt hash mismatch')
    shutil.copyfile(source/'diagnostics.json',out/'diagnostics.json')
    metadata=json.loads(json.dumps(old))
    metadata['density_frames']=frames
    metadata['density_encoding_version']=ENCODING
    metadata['density_encoding']='u16LEindex+bitoccupancy+active*u32LEtaggedpointers; lowbit1ranked/0dense; byteoffsetrelativeframealigned4; ranked16u32mask+8u16prefix+nonzeroR8padded4; dense512R8'
    metadata['products']['density.bin']={'bytes':len(payload),'sha256':sha(out/'density.bin')}
    metadata['repack']={'recipe':'scripts/cloud_source_proof/ranked.py','recipe_sha256':sha(__file__),
        'original_manifest_sha256':sha(source/'cloudclip.json'),'original_density_sha256':sha(source/'density.bin'),
        'original_optical_receipt_sha256':receipt['sha256'],
        'original_total_source_bytes':old['admitted_bytes_actual'],
        'lossless_verification':'every original R8 frame codehash preserved, all unique frames independentlydecoded bitwiseexact; sameknots/sigma/cadence/opticalreceipt',
        'ranked_bricks':ranked_count,'dense_bricks':dense_count,'wall_s':time.perf_counter()-begin}
    old_byte_failures={'runtimeheader+source exceeds11MiB','runtimeheader exceeds128KiB'}
    metadata['prior_rejection_reasons']=old['rejection_reasons']
    metadata['rejection_reasons']=[r for r in old['rejection_reasons'] if r not in old_byte_failures]
    metadata['qualification']='rejected' if metadata['rejection_reasons'] else 'declaredquarter/quantization/boundary/byte gates passed; visual/unsampledtime/engine/device budgets unqualified'
    codec.write_manifest(out/'cloudclip.json',metadata)
    if metadata['metadata_actual_bytes']>131072:metadata['rejection_reasons'].append('runtimeheader exceeds128KiB')
    if metadata['admitted_bytes_actual']>11*1024**2:metadata['rejection_reasons'].append('runtimeheader+source exceeds11MiB')
    if metadata['rejection_reasons']:metadata['qualification']='rejected'
    codec.write_manifest(out/'cloudclip.json',metadata)
    # Independent mixed reader verifies every timestamp against original code data.
    product=read_product(out)
    for index,record in enumerate(old['density_frames']):
        part=blob[record['offset']:record['offset']+record['bytes']]
        codes=codec.unpack_density(part,old['density_side'],record)
        original=codes.astype(np.float32)*np.float32(old['runtime_sigma_max_per_m']/255)
        if not np.array_equal(product['frames'][index],original):raise ValueError('full original/mixed reader mismatch')
    print('RANKED_REPACK_COMPLETE',json.dumps({'frames':len(frames),'unique_frames':len(seen),
        'binary_bytes':len(payload),'header_bytes':metadata['metadata_actual_bytes'],
        'total_source_MiB':metadata['admitted_bytes_actual']/1024**2,
        'ranked_bricks':ranked_count,'dense_bricks':dense_count,
        'qualification':metadata['qualification'],'rejection_reasons':metadata['rejection_reasons']}),flush=True)
    return metadata


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True);parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args();repack(args.source,args.out)


if __name__=='__main__':main()
