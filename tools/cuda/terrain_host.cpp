// Host-only arithmetic contract harness. Does not emulate a CUDA driver, device
// execution, scheduling, compilation by NVRTC, or synchronization.
#include <cstdint>
#include <fstream>
#include <vector>
#define __device__
#define __global__
struct Index { unsigned int x; };
static Index blockIdx{}, blockDim{64}, threadIdx{};
#include "../../crates/voxy_cuda/src/terrain.cu"
int main(int argc, char** argv) {
    if (argc != 3) return 2;
    constexpr std::size_t count = 12 + 1024 * 26 + 32768;
    static_assert(sizeof(U32) == 4 && sizeof(U64) == 8 && sizeof(I64) == 8);
    std::vector<U32> words(count);
    std::ifstream input(argv[1], std::ios::binary);
    input.read(reinterpret_cast<char*>(words.data()), std::streamsize(count * 4));
    if (!input || input.peek() != std::char_traits<char>::eof()) return 3;
    for (unsigned int block = 0; block < 16; ++block) {
        blockIdx.x = block;
        for (unsigned int thread = 0; thread < 64; ++thread) {
            threadIdx.x = thread;
            procedural_terrain(words.data());
        }
    }
    std::ofstream output(argv[2], std::ios::binary);
    output.write(reinterpret_cast<const char*>(words.data()), std::streamsize(count * 4));
    return output ? 0 : 4;
}
