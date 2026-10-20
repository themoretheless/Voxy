//! Every mip of the actual GPU owner versus independent CPU reduction.
pub(super) fn run(device:&wgpu::Device,queue:&wgpu::Queue,texture:&wgpu::Texture,depth:&[f32],width:u32,height:u32) {
 run_with_leaf_extent(device,queue,texture,depth,width,height,8);
}
pub(super) fn run_with_leaf_extent(device:&wgpu::Device,queue:&wgpu::Queue,texture:&wgpu::Texture,depth:&[f32],width:u32,height:u32,leaf:u32) {
 assert!(leaf==1||leaf==8);
 let(mut w,mut h)=(texture.width(),texture.height());let mut expected=Vec::new();
 for y in 0..h {for x in 0..w {let mut range=[1f32,0f32];for dy in 0..leaf {for dx in 0..leaf {let d=depth[((y*leaf+dy).min(height-1)*width+(x*leaf+dx).min(width-1)) as usize];if !d.is_finite()||!(0. ..=1.).contains(&d) {range=[0.,1.];}else {range[0]=range[0].min(d);if d<1. {range[1]=range[1].max(d);}}}}expected.push(range);}}
 let mut texels=0usize;
 for mip in 0..texture.mip_level_count() {
  let stride=(w*8).div_ceil(256)*256;let read=device.create_buffer(&wgpu::BufferDescriptor {label:Some("exact hierarchy mip readback"),size:u64::from(stride*h),usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});let mut encoder=device.create_command_encoder(&Default::default());encoder.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo {texture,mip_level:mip,origin:wgpu::Origin3d::ZERO,aspect:wgpu::TextureAspect::All},wgpu::TexelCopyBufferInfo {buffer:&read,layout:wgpu::TexelCopyBufferLayout {offset:0,bytes_per_row:Some(stride),rows_per_image:Some(h)}},wgpu::Extent3d {width:w,height:h,depth_or_array_layers:1});let submission=queue.submit([encoder.finish()]);let(tx,rx)=std::sync::mpsc::channel();read.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();let data=read.slice(..).get_mapped_range().unwrap();for y in 0..h {for x in 0..w {for channel in 0..2 {let offset=(y*stride+x*8+channel*4) as usize;assert_eq!(u32::from_le_bytes(data[offset..offset+4].try_into().unwrap()),expected[(y*w+x) as usize][channel as usize].to_bits(),"hierarchy mip={mip} pixel={x},{y} channel={channel}");}}}texels+=expected.len();drop(data);read.unmap();
  let(nw,nh)=((w/2).max(1),(h/2).max(1));let mut next=Vec::new();for y in 0..nh {for x in 0..nw {let mut r=[1f32,0f32];for dy in 0..2 {for dx in 0..2 {let c=expected[((y*2+dy).min(h-1)*w+(x*2+dx).min(w-1)) as usize];r[0]=r[0].min(c[0]);r[1]=r[1].max(c[1]);}}next.push(r);}}expected=next;w=nw;h=nh;
 }
 println!("GPU_EXACT_DEPTH_MIPS leaf_extent={leaf} source={}x{} base={}x{} levels={} texels={texels} both_channels_bit_exact=true padded_borders_preserved=true",width,height,texture.width(),texture.height(),texture.mip_level_count());
}
