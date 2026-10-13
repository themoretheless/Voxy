// Preserve the original addition tree and order exactly.
fn reduce_sum(lane:u32) {
    for(var stride=128u;stride>0u;stride/=2u) {
        if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
        workgroupBarrier();
    }
}
fn reduce_max(lane:u32) {
    for(var stride=128u;stride>0u;stride/=2u) {
        if lane<stride {partial[lane].x=max(partial[lane].x,partial[lane+stride].x);}
        workgroupBarrier();
    }
}
fn reduce_rows(lane:u32) {
    for(var stride=16u;stride>0u;stride/=2u) {
        if lane%32u<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
        workgroupBarrier();
    }
}
