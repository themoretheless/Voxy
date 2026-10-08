//! Runtime operand qualification for compensated Cosserat GPU arithmetic.
#[path="support/compensated_math.rs"] mod arithmetic;
use voxy_render::GraphicsOptions;
fn main()->Result<(),Box<dyn std::error::Error>> {
    let instance=GraphicsOptions::default().create_instance();
    let adapter=pollster::block_on(instance.request_adapter(&Default::default()))?;
    println!("ADAPTER {:?}",adapter.get_info());
    let (device,queue)=pollster::block_on(adapter.request_device(&Default::default()))?;
    arithmetic::qualify(&device,&queue)
}
