//! Physical CUDA broadphase parity; no graphics or CPU fallback.
#[path = "support/device_argument.rs"]
mod device_argument;
use voxy_cuda::CudaCompute;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordinal = device_argument::parse(std::env::args().skip(1))?;
    let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
    println!("CUDA voxel regions: {:?}", compute.capabilities()?);
    let dimensions = [7, 9, 11];
    let cells: Vec<u32> = (0..693).map(|i| i % 4).collect();
    let regions: Vec<[u32; 6]> = (0..257_u32)
        .map(|i| {
            let min = [i % 7, i * 3 % 9, i * 5 % 11];
            let max: [u32; 3] = std::array::from_fn(|a| min[a] + i % (dimensions[a] - min[a]));
            [min[0], min[1], min[2], max[0], max[1], max[2]]
        })
        .collect();
    let results = compute.voxel_regions(dimensions, &cells, &regions)?;
    for (region, actual) in regions.iter().zip(results) {
        let mut expected = [0, u32::MAX, u32::MAX];
        for (index, &class) in cells.iter().enumerate() {
            let index = u32::try_from(index)?;
            let pos = [index / 99, index / 11 % 9, index % 11];
            if (0..3).any(|a| pos[a] < region[a] || pos[a] > region[a + 3]) {
                continue;
            }
            if class == 1 {
                expected[0] += 1;
                expected[1] = expected[1].min(index);
            }
            if class >= 2 {
                expected[2] = expected[2].min(index);
            }
        }
        assert_eq!(actual, expected);
    }
    assert!(compute.voxel_regions([0, 9, 11], &cells, &regions).is_err());
    assert_eq!(
        compute.voxel_regions([1; 3], &[0], &[[0; 6]])?,
        [[0, u32::MAX, u32::MAX]]
    );
    println!("PASS: CUDA 257 voxel regions exact CPU counts/candidates/faults and recovery");
    Ok(())
}
