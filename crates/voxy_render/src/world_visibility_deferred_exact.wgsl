// One exact arena per worker, independent of input capacity. Queue count/cursor
// are reset by encoded clears before the fast pass, never by the CPU selector.
@compute @workgroup_size(1)
fn main_visibility_deferred_exact(@builtin(global_invocation_id) id:vec3<u32>) {
 loop {
  let index=deferred_take();if index==0xffffffffu {return;}
  visibility_process_worker(index,5u,id.x);
 }
}
