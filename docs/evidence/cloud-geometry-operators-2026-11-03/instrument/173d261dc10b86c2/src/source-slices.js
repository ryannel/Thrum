import { HEAD_STRIDE } from "./thermals.js";
// Requested source observer only. Native voxel planes never enter the live frame path.
export function sourceSections(q,dims,bounds,packed) {
  const [nx,ny,nz]=dims, head=[];
  for(let i=0;i<packed.length;i+=HEAD_STRIDE)
    if((packed[i+4]>.01||packed[i+15]>.01)&&packed[i+1]>4000&&packed[i+1]<8500){head.push(...packed.subarray(i,i+HEAD_STRIDE));break;}
  if(!head.length)return null;
  const pitch=dims.map((n,j)=>(bounds[1][j]-bounds[0][j])/n);
  const cell=(v,j)=>Math.max(0,Math.min(dims[j]-1,Math.floor((v-bounds[0][j])/pitch[j])));
  const at=(x,y,z)=>q[(z*ny+y)*nx+x];
  const cx=cell(head[0],0),cy=cell(head[1],1),cz=cell(head[2],2);
  const xy=[],xz=[],axis=[];
  for(let y=0;y<ny;y++)for(let x=0;x<nx;x++)xy.push(at(x,y,cz));
  for(let z=0;z<nz;z++)for(let x=0;x<nx;x++)xz.push(at(x,cy,z));
  for(let y=0;y<ny;y++)axis.push([bounds[0][1]+(y+.5)*pitch[1],at(cx,y,cz)]);
  return {scope:'Native total-q planes; fixed voxel indices through first developed head; not interpolated geometry or visible-light acceptance',head,voxelPitch:pitch,bounds,xy:{dims:[nx,ny],fixedZ:cz,values:xy},xz:{dims:[nx,nz],fixedY:cy,values:xz},axis};
}
