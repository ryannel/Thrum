use wgpu::naga;
fn main(){
 let args:Vec<_>=std::env::args().collect();let source=std::fs::read_to_string(&args[1]).unwrap();
 let module=naga::front::wgsl::parse_str(&source).unwrap();
 let info=naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module).unwrap();
 let options=naga::back::msl::Options{lang_version:(2,4),fake_missing_bindings:true,..Default::default()};
 let pipeline=naga::back::msl::PipelineOptions {entry_point:Some((naga::ShaderStage::Vertex,args[3].clone())),..Default::default()};
 let (msl,_) = naga::back::msl::write_string(&module,&info,&options,&pipeline).unwrap();
 std::fs::write(&args[2],msl).unwrap();
 for ep in &module.entry_points {println!("{}: {:?}",ep.name,ep.function.result.as_ref().unwrap().binding);}
}
