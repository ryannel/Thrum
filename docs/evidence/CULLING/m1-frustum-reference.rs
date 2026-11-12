#[path = "../../examples/v1-game/src/field.rs"] mod field;
#[path = "../../examples/v1-game/src/field_meshes.rs"] mod field_meshes;
use thrum_render::synthetic::{SyntheticMeshChoice, SyntheticView};
fn main() {
    let meshes=field_meshes::selection();
    let mut rejected=0; let mut unsafe_rejections=0; let mut nearest=f64::INFINITY;
    let mut witness=(0,0,0,0.0,0.0);
    for tick in 850..=880 {
        let camera=field::camera(tick,1.0);
        let target=[camera.eye[0]+camera.heading.cos()*10.0,camera.eye[1],camera.eye[2]+camera.heading.sin()*10.0];
        let m=SyntheticView::perspective(camera.eye,target,[0.0,1.0,0.0],65.0_f32.to_radians(),640.0/360.0,0.1,1200.0).unwrap().view_proj_columns();
        let row=|r:usize|std::array::from_fn::<_,4,_>(|c|m[c][r]);
        let r0=row(0);let r1=row(1);let r2=row(2);let r3=row(3);
        let planes=[std::array::from_fn(|i|r3[i]+r0[i]),std::array::from_fn(|i|r3[i]-r0[i]),std::array::from_fn(|i|r3[i]+r1[i]),std::array::from_fn(|i|r3[i]-r1[i]),std::array::from_fn(|i|r3[i]-r2[i]),r2];
        for slot in 0..field::ENTITY_COUNT {
            let mut entity=field::entity(slot);
            for _ in 0..tick {for i in 0..3 {entity.position[i]+=entity.velocity[i]/60.0;}}
            let (mut basis,_)=field::basis(&entity);
            if field_meshes::mirrored(slot) {basis[0]=basis[0].map(|v|-v);}
            let translation=entity.position.map(|v|v as f32);
            let (lo,hi)=match meshes.slots[slot] {
                SyntheticMeshChoice::Cube=>([-1.0_f32;3],[1.0_f32;3]),
                SyntheticMeshChoice::Mesh(mesh)=>{
                    let mut lo=[f32::INFINITY;3];let mut hi=[f32::NEG_INFINITY;3];
                    for v in &meshes.meshes[mesh as usize].vertices {for i in 0..3 {lo[i]=lo[i].min(v.position[i]);hi[i]=hi[i].max(v.position[i]);}}
                    (lo,hi)
                }
            };
            let center: [f32;3]=std::array::from_fn(|i|(lo[i]+hi[i])*0.5);
            let extent: [f32;3]=std::array::from_fn(|i|(hi[i]-lo[i])*0.5);
            let wc: [f32;3]=std::array::from_fn(|r|basis[0][r]*center[0]+basis[1][r]*center[1]+basis[2][r]*center[2]+translation[r]);
            let we: [f32;3]=std::array::from_fn(|r|basis[0][r].abs()*extent[0]+basis[1][r].abs()*extent[1]+basis[2][r].abs()*extent[2]);
            for (plane,p) in planes.iter().enumerate() {
                let distance=p[0]*wc[0]+p[1]*wc[1]+p[2]*wc[2]+p[3];
                let radius=p[0].abs()*we[0]+p[1].abs()*we[1]+p[2].abs()*we[2];
                if distance+radius*1.0001+1e-5*p[3].abs()>=0.0 {continue;}
                rejected+=1;
                let mut support=f64::from(p[3]); let mut magnitude=f64::from(p[3]).abs();
                for r in 0..3 {
                    let c=(0..3).map(|a|f64::from(basis[a][r])*(f64::from(lo[a])+f64::from(hi[a]))*0.5).sum::<f64>()+f64::from(translation[r]);
                    let e=(0..3).map(|a|f64::from(basis[a][r]).abs()*(f64::from(hi[a])-f64::from(lo[a]))*0.5).sum::<f64>();
                    support+=f64::from(p[r])*c+f64::from(p[r]).abs()*e;
                    magnitude+=f64::from(p[r]).abs()*(c.abs()+e);
                }
                let rounding=32.0*f64::from(f32::EPSILON)*magnitude;
                let clearance=-support-rounding;
                if clearance<nearest {nearest=clearance;witness=(tick,slot,plane,support,rounding);}
                unsafe_rejections+=u64::from(clearance<=0.0);
                break;
            }
        }
    }
    println!("CPU diagnostic ticks850..880: rejected {rejected}, unproved under 32-epsilon allowance {unsafe_rejections}; nearest clearance {nearest}; witness {witness:?}");
}
