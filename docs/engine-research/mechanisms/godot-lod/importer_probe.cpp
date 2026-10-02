// Voxy research fixture; builds the pinned upstream simplifier separately.
// This is not a Voxy importer or a surface-error certification test.
#include "meshoptimizer.h"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <limits>
#include <stdexcept>
#include <vector>

static void require(bool condition, const char* message) {
    if (!condition) throw std::runtime_error(message);
}

static void export_obj(const std::filesystem::path& path, const std::vector<float>& positions,
    const std::vector<float>& attributes, const std::vector<unsigned>& indices) {
    require(!std::filesystem::exists(path), "refusing to overwrite an exported mesh");
    std::ofstream output(path);
    require(bool(output), "cannot create exported mesh");
    output.precision(std::numeric_limits<float>::max_digits10);
    for (size_t i = 0; i < positions.size(); i += 3)
        output << "v " << positions[i] << ' ' << positions[i+1] << ' ' << positions[i+2] << '\n';
    for (size_t i = 0; i < positions.size(); i += 3)
        output << "vt " << positions[i] << ' ' << positions[i+1] << '\n';
    for (size_t i = 0; i < attributes.size(); i += 6)
        output << "vn " << attributes[i] << ' ' << attributes[i+1] << ' ' << attributes[i+2] << '\n';
    for (size_t i = 0; i < indices.size(); i += 3) {
        output << "f";
        for (size_t j = 0; j < 3; ++j) {
            unsigned vertex = indices[i+j]+1;
            output << ' ' << vertex << '/' << vertex << '/' << vertex;
        }
        output << '\n';
    }
    output.close();
    require(bool(output), "exported mesh write failed");
}

int main(int argc, char** argv) {
    require(argc == 1 || argc == 2, "usage: probe [existing-export-directory]");
    constexpr unsigned side = 17;
    constexpr float pi = 3.14159265358979323846f;
    std::vector<float> positions, attributes;
    std::vector<unsigned> indices;
    for (unsigned y = 0; y < side; ++y) {
        for (unsigned x = 0; x < side; ++x) {
            float u = float(x) / float(side - 1), v = float(y) / float(side - 1);
            positions.insert(positions.end(), {u, v, 0.2f * std::sin(pi*u) * std::sin(pi*v)});
            float nx = -0.2f*pi*std::cos(pi*u)*std::sin(pi*v);
            float ny = -0.2f*pi*std::sin(pi*u)*std::cos(pi*v);
            float length = std::sqrt(nx*nx + ny*ny + 1.0f);
            attributes.insert(attributes.end(), {nx/length, ny/length, 1.0f/length, 1.0f, 1.0f, 1.0f});
        }
    }
    for (unsigned y = 0; y + 1 < side; ++y) {
        for (unsigned x = 0; x + 1 < side; ++x) {
            unsigned a = y*side+x, b = a+1, c = a+side, d = c+1;
            indices.insert(indices.end(), {a, b, d, a, d, c});
        }
    }
    const auto original_positions = positions;
    const auto original_attributes = attributes;
    if (argc == 2) export_obj(std::filesystem::path(argv[1])/"base.obj", positions, attributes, indices);
    const float weights[6] = {1, 1, 1, 1, 1, 1};
    float scale = meshopt_simplifyScale(positions.data(), side*side, 3*sizeof(float));
    float accumulated = 0;
    unsigned published = 0;
    bool allow_prune = true;
    std::puts("level,input_indices,target_indices,output_indices,step_metric,selection_metric,object_metric");
    while (indices.size() > 24) {
        size_t before = indices.size(), target = std::max((before / 3 / 2) * 3, size_t(12));
        std::vector<unsigned> next(before);
        unsigned options = meshopt_SimplifySparse | meshopt_SimplifyLockBorder;
        if (allow_prune) options |= meshopt_SimplifyPrune;
        float step = 0;
        size_t count = meshopt_simplifyWithAttributes(next.data(), indices.data(), before,
            positions.data(), side*side, 3*sizeof(float), attributes.data(), 6*sizeof(float),
            weights, 6, nullptr, target, 1.0f, options, &step);
        if (!count && allow_prune) { allow_prune = false; continue; }
        require(count <= before && count % 3 == 0, "invalid triangle output size");
        require(std::isfinite(step) && step >= 0, "invalid simplifier metric");
        accumulated = std::max(accumulated * 1.5f, step);
        if (!count || count >= before * 0.75f || accumulated > 1.0f) break;
        next.resize(count);
        std::vector<bool> used(side*side);
        for (unsigned index : next) {
            require(index < side*side, "foreign vertex index");
            used[index] = true;
        }
        for (unsigned y = 0; y < side; ++y)
            for (unsigned x = 0; x < side; ++x)
                if (x == 0 || y == 0 || x + 1 == side || y + 1 == side)
                    require(used[y*side+x], "locked boundary vertex disappeared");
        require(positions == original_positions && attributes == original_attributes,
            "source streams mutated");
        std::printf("%u,%zu,%zu,%zu,%.9g,%.9g,%.9g\n", ++published, before, target,
            count, double(step), double(accumulated), double(accumulated*scale));
        if (argc == 2) export_obj(std::filesystem::path(argv[1])/("lod-"+std::to_string(published)+".obj"), positions, attributes, next);
        indices = std::move(next);
    }
    require(published > 1, "fixture did not exercise a LOD chain");
}
