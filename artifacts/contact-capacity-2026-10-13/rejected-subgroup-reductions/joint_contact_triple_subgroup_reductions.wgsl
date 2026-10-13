// Qualification only: caller checks the actual subgroup mapping at entry.
// Keep the original binary addition tree; shuffle only replaces shared-memory
// transport and the five workgroup barriers within each 32-lane reduction.
fn reduce_rows(lane:u32) {
    var value=partial[lane];
    for(var stride=16u;stride>0u;stride/=2u) {
        let other=subgroupShuffleDown(value,stride);
        if lane%32u<stride {value=add(value,other);}
    }
    partial[lane]=value;workgroupBarrier();
}
fn reduce_sum(lane:u32) {
    for(var stride=128u;stride>=32u;stride/=2u) {
        if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
        workgroupBarrier();
    }
    reduce_rows(lane);
}
fn reduce_max(lane:u32) {
    for(var stride=128u;stride>=32u;stride/=2u) {
        if lane<stride {partial[lane].x=max(partial[lane].x,partial[lane+stride].x);}
        workgroupBarrier();
    }
    var value=partial[lane].x;
    for(var stride=16u;stride>0u;stride/=2u) {
        let other=subgroupShuffleDown(value,stride);
        if lane%32u<stride {value=max(value,other);}
    }
    partial[lane].x=value;workgroupBarrier();
}
