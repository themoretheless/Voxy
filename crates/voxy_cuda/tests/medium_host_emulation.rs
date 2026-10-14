//! Plan step B4 without an NVIDIA device: `medium.cu` compiled by the host C++
//! compiler behind a tiny CUDA shim and driven thread-by-thread over the same
//! packed words the driver path uploads. The kernel uses no shared memory,
//! atomics or barriers, so a serial sweep over `blockIdx * blockDim + threadIdx`
//! is an exact model of one launch. The host loop mirrors
//! `CudaCompute::medium_transport` (parity word per substep, result parity).
//! Skips when no C++ compiler is found.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use physics::liquid::{FiniteDropletGasGrid, VaporCell};
use physics::medium::{
    MediumSpecies, MediumSpeciesProperties, MediumStepControl, ParticipatingMedium,
};
use voxy_cuda::{pack_medium_job, unpack_medium_result};

const SHIM: &str = r#"
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <vector>
#define __device__
#define __global__
struct Dim3 { unsigned int x, y, z; };
static Dim3 threadIdx, blockIdx, blockDim;
static inline float __uint_as_float(unsigned int word) {
    float value;
    static_assert(sizeof(value) == sizeof(word), "word size");
    __builtin_memcpy(&value, &word, sizeof(value));
    return value;
}
"#;

const DRIVER: &str = r#"
int main(int argc, char** argv) {
    if (argc != 3) return 2;
    FILE* in = std::fopen(argv[1], "rb");
    if (!in) return 3;
    std::vector<unsigned int> words;
    unsigned int word;
    while (std::fread(&word, sizeof(word), 1, in) == 1) words.push_back(word);
    std::fclose(in);
    if (words.size() < 16) return 4;
    const unsigned int cells = words[0];
    const unsigned int substeps = words[4];
    blockDim = { 256u, 1u, 1u };
    for (unsigned int step = 0; step < substeps; ++step) {
        words[15] = step % 2u;
        const unsigned int blocks = (cells + 255u) / 256u;
        for (unsigned int block = 0; block < blocks; ++block) {
            blockIdx = { block, 0u, 0u };
            for (unsigned int thread = 0; thread < 256u; ++thread) {
                threadIdx = { thread, 0u, 0u };
                medium_substep(words.data());
            }
        }
    }
    words[15] = substeps % 2u;
    FILE* out = std::fopen(argv[2], "wb");
    if (!out) return 5;
    const size_t written = std::fwrite(words.data(), sizeof(unsigned int), words.size(), out);
    std::fclose(out);
    return written == words.size() ? 0 : 6;
}
"#;

fn compiler() -> Option<String> {
    let candidates = std::env::var("CXX")
        .ok()
        .into_iter()
        .chain(["c++", "clang++", "g++"].map(str::to_owned));
    candidates.into_iter().find(|name| {
        Command::new(name)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

fn build_emulator(dir: &Path) -> Option<PathBuf> {
    let compiler = compiler()?;
    let source = dir.join("medium_emulation.cpp");
    let kernel = include_str!("../src/medium.cu");
    fs::write(&source, format!("{SHIM}\n{kernel}\n{DRIVER}")).unwrap();
    let binary = dir.join("medium_emulation");
    let output = Command::new(&compiler)
        .args(["-std=c++17", "-O1", "-o"])
        .arg(&binary)
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{compiler} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(binary)
}

fn run_emulator(binary: &Path, dir: &Path, words: &[u32]) -> Vec<u32> {
    let input = dir.join("job.bin");
    let output = dir.join("result.bin");
    fs::write(
        &input,
        words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<u8>>(),
    )
    .unwrap();
    let status = Command::new(binary)
        .arg(&input)
        .arg(&output)
        .status()
        .unwrap();
    assert!(status.success(), "emulator exited with {status}");
    fs::read(&output)
        .unwrap()
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

struct Case {
    shape: [usize; 3],
    up: [f64; 3],
    settling: f64,
    diffusivity: f64,
    dt: f64,
}

fn reference(case: &Case) -> (ParticipatingMedium, MediumSpeciesProperties, usize) {
    let h = 0.1;
    let volume = h * h * h;
    let n = case.shape;
    let cells = (0..n[0] * n[1] * n[2])
        .map(|i| VaporCell {
            mass: 1.2 * volume,
            volume,
            temperature: 300.,
            velocity: [
                0.2,
                -0.1 * ((i % 16) as f64 * 0.3).cos(),
                0.15 * ((i % 7) as f64 * 0.5).sin(),
            ],
            specific_heat_cv: 718.,
        })
        .collect();
    let species = MediumSpeciesProperties {
        kind: MediumSpecies::Dust,
        mass_extinction_m2_kg: 1.,
        albedo: 0.5,
        settling_velocity_m_s: case.settling,
        diffusivity_m2_s: case.diffusivity,
        buoyancy_per_kg_m3: 0.,
    };
    let mut medium = ParticipatingMedium::new(
        FiniteDropletGasGrid::new([0.; 3], [h; 3], n, cells).unwrap(),
        vec![species],
    )
    .unwrap();
    let extent = n.map(|k| k as f64 * h);
    for i in 0..5 {
        let x = (0.15 + 0.1 * i as f64).min(extent[0] - 0.05);
        medium
            .deposit(0, [x, extent[1] - 0.05, 0.5 * extent[2]], 0.5)
            .unwrap();
        medium.deposit(0, [x, 0.05, 0.5 * extent[2]], 0.2).unwrap();
    }
    (medium, species, 0)
}

#[test]
fn host_compiled_cuda_medium_kernel_matches_the_cpu_reference() {
    let dir =
        std::env::temp_dir().join(format!("voxy_cuda_medium_emulation_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let Some(binary) = build_emulator(&dir) else {
        eprintln!("no C++ compiler found; skipping the host emulation oracle");
        return;
    };
    let cases = [
        Case {
            shape: [16; 3],
            up: [0., 1., 0.],
            settling: 0.3,
            diffusivity: 2e-4,
            dt: 0.5,
        },
        Case {
            shape: [12, 9, 7],
            up: [0., 0., 1.],
            settling: 0.,
            diffusivity: 0.,
            dt: 0.25,
        },
        Case {
            shape: [10, 1, 6],
            up: [0., 0., -1.],
            settling: 0.2,
            diffusivity: 5e-4,
            dt: 0.4,
        },
    ];
    for case in &cases {
        let (mut medium, species, _) = reference(case);
        let h = 0.1f32;
        let velocity: Vec<[f32; 3]> = medium
            .gas()
            .cells()
            .iter()
            .map(|c| c.velocity.map(|v| v as f32))
            .collect();
        let mass: Vec<f32> = medium.mass_kg(0).iter().map(|m| *m as f32).collect();
        let before: f64 = medium.mass_kg(0).iter().sum();
        let report = medium
            .advance(
                case.dt,
                MediumStepControl {
                    advance_gas: false,
                    // The reference derives `up` from gravity.
                    gravity: case.up.map(|u| -9.81 * u),
                    ..MediumStepControl::default()
                },
            )
            .unwrap();
        let words = pack_medium_job(
            case.shape,
            [h; 3],
            case.up.map(|u| u as f32),
            report.substeps,
            case.dt as f32,
            species.settling_velocity_m_s as f32,
            species.diffusivity_m2_s as f32,
            &velocity,
            &mass,
        )
        .unwrap();
        let output = run_emulator(&binary, &dir, &words);
        assert_eq!(output.len(), words.len());
        let (emulated, settled) = unpack_medium_result(&output).unwrap();
        let max_difference = emulated
            .iter()
            .zip(medium.mass_kg(0))
            .map(|(g, c)| (f64::from(*g) - c).abs())
            .fold(0., f64::max);
        let emulated_total: f64 = emulated.iter().map(|m| f64::from(*m)).sum();
        eprintln!(
            "shape {:?}: substeps {}, max cell difference {max_difference:.3e}, settled {} vs {}",
            case.shape, report.substeps, settled, report.settled_kg[0]
        );
        assert!(max_difference <= 1e-4 * before, "{max_difference}");
        assert!((f64::from(settled) - report.settled_kg[0]).abs() <= 1e-4 * before);
        // Mass leaves only through the settling face, in both implementations.
        assert!((emulated_total + f64::from(settled) - before).abs() <= 1e-4 * before);
    }
    fs::remove_dir_all(&dir).unwrap();
}
