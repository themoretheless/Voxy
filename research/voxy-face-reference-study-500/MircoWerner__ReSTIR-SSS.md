# Facial rendering source scan: MircoWerner/ReSTIR-SSS

Repository: https://github.com/MircoWerner/ReSTIR-SSS

Pinned source revision: c209f2b72b85cc51aa2577c14534952154ae4313

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "VkRaven/LICENSE"]

Topic paths: {"sss": ["ReSTIRSSS/.clang-format", "ReSTIRSSS/.clang-tidy", "ReSTIRSSS/CMakeLists.txt", "ReSTIRSSS/include/denoiser/DenoiserOptiX.h", "ReSTIRSSS/include/restirsss/Raydata.h", "ReSTIRSSS/include/restirsss/Rayscenes.h", "ReSTIRSSS/include/restirsss/Raystructs.h", "ReSTIRSSS/include/restirsss/ReSTIRSSS.h", "ReSTIRSSS/include/restirsss/ReSTIRSSSEvaluation.h", "ReSTIRSSS/include/restirsss/ReSTIRSSSEvaluationTiming.h", "ReSTIRSSS/include/restirsss/ReSTIRSSSVideo.h", "ReSTIRSSS/include/restirsss/passes/PassDebug.h", "ReSTIRSSS/include/restirsss/passes/PassGBuffer.h", "ReSTIRSSS/include/restirsss/passes/PassPathtrace.h", "ReSTIRSSS/include/restirsss/passes/PassSurface.h", "ReSTIRSSS/include/restirsss/passes/PassTonemapper.h", "ReSTIRSSS/include/restirsss/passes/ReSTIRSSSPassCandidateGeneration.h", "ReSTIRSSS/include/restirsss/passes/ReSTIRSSSPassShade.h", "ReSTIRSSS/include/restirsss/passes/ReSTIRSSSPassSpatialReuse.h", "ReSTIRSSS/include/restirsss/passes/ReSTIRSSSPassTemporalReuse.h", "ReSTIRSSS/resources/environment/white.png", "ReSTIRSSS/resources/scenes/ajax.xml", "ReSTIRSSS/resources/scenes/ajaxmanylights.xml", "ReSTIRSSS/resources/scenes/asiandragon.xml", "ReSTIRSSS/resources/scenes/flattest.xml", "ReSTIRSSS/resources/scenes/lteorb.xml", "ReSTIRSSS/resources/shaders/arealight.glsl", "ReSTIRSSS/resources/shaders/bxdf/bsdf/bsdf.glsl", "ReSTIRSSS/resources/shaders/bxdf/bsdf/disney_bsdf.glsl", "ReSTIRSSS/resources/shaders/bxdf/bsdf/disney_diffuse.glsl"], "shader": ["ReSTIRSSS/resources/shaders/arealight.glsl", "ReSTIRSSS/resources/shaders/bxdf/bsdf/bsdf.glsl", "ReSTIRSSS/resources/shaders/bxdf/bsdf/disney_bsdf.glsl", "ReSTIRSSS/resources/shaders/bxdf/bsdf/disney_diffuse.glsl", "ReSTIRSSS/resources/shaders/bxdf/bsdf/disney_metal.glsl", "ReSTIRSSS/resources/shaders/bxdf/diffusion/sss_diffusion_profile.glsl", "ReSTIRSSS/resources/shaders/bxdf/diffusion/sss_diffusion_profile_burley.glsl", "ReSTIRSSS/resources/shaders/bxdf/diffusion/sss_sampling.glsl", "ReSTIRSSS/resources/shaders/bxdf/diffusion/sss_sampling_disk.glsl", "ReSTIRSSS/resources/shaders/bxdf/disney.glsl", "ReSTIRSSS/resources/shaders/defines.glsl", "ReSTIRSSS/resources/shaders/environment.glsl", "ReSTIRSSS/resources/shaders/main/gbuffer.glsl", "ReSTIRSSS/resources/shaders/main/passes/pass_debug.comp", "ReSTIRSSS/resources/shaders/main/passes/pass_gbuffer.comp", "ReSTIRSSS/resources/shaders/main/passes/pass_pathtrace.comp", "ReSTIRSSS/resources/shaders/main/passes/pass_surface.comp", "ReSTIRSSS/resources/shaders/main/passes/pass_tonemapper.comp", "ReSTIRSSS/resources/shaders/main/passes/restirsss_pass_candidate_generation.comp", "ReSTIRSSS/resources/shaders/main/passes/restirsss_pass_shade.comp", "ReSTIRSSS/resources/shaders/main/passes/restirsss_pass_spatial_reuse.comp", "ReSTIRSSS/resources/shaders/main/passes/restirsss_pass_temporal_reuse.comp", "ReSTIRSSS/resources/shaders/main/restirsss/probinversetest/restirsss_shift_test.glsl", "ReSTIRSSS/resources/shaders/main/restirsss/probinversetest/restirsss_target_function_test.glsl", "ReSTIRSSS/resources/shaders/main/restirsss/restirsss_defines.glsl", "ReSTIRSSS/resources/shaders/main/restirsss/restirsss_disney_bsdf.glsl", "ReSTIRSSS/resources/shaders/main/restirsss/restirsss_disney_bssrdf.glsl", "ReSTIRSSS/resources/shaders/main/restirsss/restirsss_mis.glsl", "ReSTIRSSS/resources/shaders/main/restirsss/restirsss_pixel_info.glsl", "ReSTIRSSS/resources/shaders/main/restirsss/restirsss_reservoir.glsl"], "face": ["ReSTIRSSS/include/restirsss/passes/PassSurface.h", "ReSTIRSSS/resources/shaders/main/passes/pass_surface.comp"]}

## Source file: VkRaven/raven/include/raven/passes/PassShader.h

{
  "path": "VkRaven/raven/include/raven/passes/PassShader.h",
  "blob_sha": "a3ae039be6f218e14ebdf0e3f769f1ccc3822d69",
  "sha256": "c95dc918a6947a2102c016415c3bb966f7ac5098f63a389595c33b1ba560b426",
  "lines": 509,
  "bytes": 25238,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 6,
    "shader": 21
  },
  "url": "https://github.com/MircoWerner/ReSTIR-SSS/blob/c209f2b72b85cc51aa2577c14534952154ae4313/VkRaven/raven/include/raven/passes/PassShader.h",
  "function_signature_candidates": [
    {
      "line": 22,
      "text": "virtual void execute(const std::function<void(const vk::CommandBuffer &commandBuffer, const vk::PipelineLayout &pipelineLayout)> &recordPushConstants,"
    },
    {
      "line": 26,
      "text": "virtual void create() {"
    },
    {
      "line": 44,
      "text": "virtual void release() {"
    },
    {
      "line": 61,
      "text": "void updateSpecializationConstants(const int32_t shaderIndex, std::vector<vk::SpecializationMapEntry> specializationMapEntries, std::vector<uint32_t> specializationData) {"
    },
    {
      "line": 69,
      "text": "void setStorageBuffer(const uint32_t set, const uint32_t binding, Buffer *buffer) {"
    },
    {
      "line": 95,
      "text": "void setStorageImage(const uint32_t set, const uint32_t binding, Image *image) {"
    },
    {
      "line": 121,
      "text": "void setStorageImages(const uint32_t set, const uint32_t binding, const std::vector<Image *> &images) {"
    },
    {
      "line": 149,
      "text": "void setStorageImage(const uint32_t set, const uint32_t binding, Image *image, const uint32_t activeIndex) {"
    },
    {
      "line": 173,
      "text": "void setSamplerImage(const uint32_t set, const uint32_t binding, Image *image, const vk::ImageLayout imageLayout = vk::ImageLayout::eGeneral) {"
    },
    {
      "line": 199,
      "text": "void setSamplerImages(const uint32_t set, const uint32_t binding, const std::vector<Image *> &images, const vk::ImageLayout imageLayout = vk::ImageLayout::eGeneral) {"
    },
    {
      "line": 227,
      "text": "void setSamplerImage(const uint32_t set, const uint32_t binding, Image *image, const uint32_t activeIndex, const vk::ImageLayout imageLayout = vk::ImageLayout::eGeneral) {"
    },
    {
      "line": 251,
      "text": "void setAccelerationStructure(const uint32_t set, const uint32_t binding, const vk::AccelerationStructureKHR &as) {"
    },
    {
      "line": 278,
      "text": "[[nodiscard]] float getTime() const { return m_timeValid ? m_timeInMs : NAN; }"
    },
    {
      "line": 279,
      "text": "[[nodiscard]] float getTimeAveraged() const { return m_timeValid ? m_timeInMsAveraged : NAN; }"
    },
    {
      "line": 281,
      "text": "void invalidateTime() { m_timeValid = false; }"
    },
    {
      "line": 310,
      "text": "virtual void createPipeline() = 0;"
    },
    {
      "line": 318,
      "text": "virtual bool getPushConstantRange(vk::PushConstantRange &pushConstantRange) = 0;"
    },
    {
      "line": 320,
      "text": "void getDescriptorSets(std::vector<vk::DescriptorSet> &sets) const {"
    },
    {
      "line": 327,
      "text": "void queryTimeFromGPU() {"
    },
    {
      "line": 345,
      "text": "void createPipelineLayout() {"
    },
    {
      "line": 364,
      "text": "void createDescriptorSetLayout() {"
    },
    {
      "line": 413,
      "text": "void createDescriptorPool() {"
    },
    {
      "line": 441,
      "text": "void createDescriptorSets() {"
    },
    {
      "line": 457,
      "text": "void createUniforms() {"
    },
    {
      "line": 466,
      "text": "void releaseUniforms() {"
    },
    {
      "line": 474,
      "text": "void setUniform(Uniform *uniform) {"
    },
    {
      "line": 499,
      "text": "void createTimestampQueryPool() {"
    }
  ]
}

```text

#pragma once
#include "raven/core/Image.h"
#include "raven/core/Shader.h"

#include <memory>
#include <ranges>
#include <utility>
#include <vector>

namespace raven {
#define RAVEN_PASS_SHADER_RELEASE(passShader) \
    if (passShader)                           \
    passShader->release()

    class PassShader {
    public:
        PassShader(GPUContext *gpuContext, std::string name) : m_gpuContext(gpuContext), m_name(std::move(name)) {
        }

        virtual ~PassShader() = default;

        virtual void execute(const std::function<void(const vk::CommandBuffer &commandBuffer, const vk::PipelineLayout &pipelineLayout)> &recordPushConstants,
                             const std::function<void(const vk::CommandBuffer &commandBuffer)> &recordPipelineBarriers,
                             vk::CommandBuffer &commandBuffer) = 0;

        virtual void create() {
            m_shaders = createShaders();
            createDescriptorSetLayout();
            createDescriptorPool();
            createDescriptorSets();
            createPipelineLayout();
            vk::PipelineCacheCreateInfo pipelineCacheCreateInfo{};
            pipelineCacheCreateInfo.initialDataSize = 0;
            pipelineCacheCreateInfo.pInitialData = nullptr;
            if (m_gpuContext->m_device.createPipelineCache(&pipelineCacheCreateInfo, nullptr, &m_pipelineCache) != vk::Result::eSuccess) {
                throw std::runtime_error("Failed to create pipeline cache!");
            }
            createPipeline();

            createUniforms();
            createTimestampQueryPool();
        }

        virtual void release() {
            m_gpuContext->m_device.destroyQueryPool(m_queryPoolTimestamps);

            releaseUniforms();

            m_gpuContext->m_device.destroyDescriptorPool(m_descriptorPool);
            for (auto &descriptorSetLayout: m_descriptorSetLayouts) {
                m_gpuContext->m_device.destroyDescriptorSetLayout(descriptorSetLayout);
            }
            m_gpuContext->m_device.destroyPipeline(m_pipeline);
            m_gpuContext->m_device.destroyPipelineCache(m_pipelineCache);
            m_gpuContext->m_device.destroyPipelineLayout(m_pipelineLayout);
            for (const auto &shader: m_shaders) {
                shader->release();
            }
        }

        void updateSpecializationConstants(const int32_t shaderIndex, std::vector<vk::SpecializationMapEntry> specializationMapEntries, std::vector<uint32_t> specializationData) {
            m_shaders[shaderIndex]->updateSpecializationConstants(std::move(specializationMapEntries), std::move(specializationData));
            m_gpuContext->m_device.destroyPipeline(m_pipeline);
            createPipeline();
        }

        std::string getName() { return m_name; }

        void setStorageBuffer(const uint32_t set, const uint32_t binding, Buffer *buffer) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            vk::DescriptorBufferInfo bufferInfo{};
            bufferInfo.buffer = buffer->getBuffer();
            bufferInfo.offset = 0;
            bufferInfo.range = buffer->getSizeBytes();

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = 1;
            writeDescriptorSet.pBufferInfo = &bufferInfo;

            for (size_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                writeDescriptorSet.dstSet = m_descriptorSets[i][setIdx];
                m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
            }
        }

        void setStorageImage(const uint32_t set, const uint32_t binding, Image *image) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            vk::DescriptorImageInfo imageInfo{};
            imageInfo.imageLayout = vk::ImageLayout::eGeneral;
            imageInfo.imageView = image->getImageView();
            imageInfo.sampler = image->getSampler();

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = 1;
            writeDescriptorSet.pImageInfo = &imageInfo;

            for (size_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                writeDescriptorSet.dstSet = m_descriptorSets[i][setIdx];
                m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
            }
        }

        void setStorageImages(const uint32_t set, const uint32_t binding, const std::vector<Image *> &images) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            std::vector<vk::DescriptorImageInfo> imageInfos(images.size());
            for (uint32_t i = 0; i < images.size(); i++) {
                imageInfos[i].imageLayout = vk::ImageLayout::eGeneral;
                imageInfos[i].imageView = images[i]->getImageView();
                imageInfos[i].sampler = images[i]->getSampler();
            }

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = images.size();
            writeDescriptorSet.pImageInfo = imageInfos.data();

            for (size_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                writeDescriptorSet.dstSet = m_descriptorSets[i][setIdx];
                m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
            }
        }

        void setStorageImage(const uint32_t set, const uint32_t binding, Image *image, const uint32_t activeIndex) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            vk::DescriptorImageInfo imageInfo{};
            imageInfo.imageLayout = vk::ImageLayout::eGeneral;
            imageInfo.imageView = image->getImageView();
            imageInfo.sampler = image->getSampler();

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = 1;
            writeDescriptorSet.pImageInfo = &imageInfo;

            writeDescriptorSet.dstSet = m_descriptorSets[activeIndex][setIdx];
            m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
        }

        void setSamplerImage(const uint32_t set, const uint32_t binding, Image *image, const vk::ImageLayout imageLayout = vk::ImageLayout::eGeneral) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            vk::DescriptorImageInfo imageInfo{};
            imageInfo.imageLayout = imageLayout;
            imageInfo.imageView = image->getImageView();
            imageInfo.sampler = image->getSampler();

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = 1;
            writeDescriptorSet.pImageInfo = &imageInfo;

            for (size_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                writeDescriptorSet.dstSet = m_descriptorSets[i][setIdx];
                m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
            }
        }

        void setSamplerImages(const uint32_t set, const uint32_t binding, const std::vector<Image *> &images, const vk::ImageLayout imageLayout = vk::ImageLayout::eGeneral) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            std::vector<vk::DescriptorImageInfo> imageInfos(images.size());
            for (uint32_t i = 0; i < images.size(); i++) {
                imageInfos[i].imageLayout = imageLayout;
                imageInfos[i].imageView = images[i]->getImageView();
                imageInfos[i].sampler = images[i]->getSampler();
            }

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = images.size();
            writeDescriptorSet.pImageInfo = imageInfos.data();

            for (size_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                writeDescriptorSet.dstSet = m_descriptorSets[i][setIdx];
                m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
            }
        }

        void setSamplerImage(const uint32_t set, const uint32_t binding, Image *image, const uint32_t activeIndex, const vk::ImageLayout imageLayout = vk::ImageLayout::eGeneral) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            vk::DescriptorImageInfo imageInfo{};
            imageInfo.imageLayout = imageLayout;
            imageInfo.imageView = image->getImageView();
            imageInfo.sampler = image->getSampler();

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = 1;
            writeDescriptorSet.pImageInfo = &imageInfo;

            writeDescriptorSet.dstSet = m_descriptorSets[activeIndex][setIdx];
            m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
        }

        void setAccelerationStructure(const uint32_t set, const uint32_t binding, const vk::AccelerationStructureKHR &as) {
            const uint32_t setIdx = m_descriptorSetToIndex[set];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[binding]];
            assert(set == set_number);
            assert(binding == bindingLayout.binding);

            vk::WriteDescriptorSetAccelerationStructureKHR descriptorSetAS{};
            descriptorSetAS.accelerationStructureCount = 1;
            descriptorSetAS.pAccelerationStructures = &as;

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = binding;
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = 1;
            writeDescriptorSet.pNext = &descriptorSetAS;

            for (size_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                writeDescriptorSet.dstSet = m_descriptorSets[i][setIdx];
                m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
            }
        }

        std::shared_ptr<Uniform> getUniform(const uint32_t set, const uint32_t binding) { return m_uniforms[set][binding]; }

        [[nodiscard]] float getTime() const { return m_timeValid ? m_timeInMs : NAN; }
        [[nodiscard]] float getTimeAveraged() const { return m_timeValid ? m_timeInMsAveraged : NAN; }

        void invalidateTime() { m_timeValid = false; }

    protected:
        GPUContext *m_gpuContext;
        std::string m_name;

        std::vector<std::shared_ptr<Shader>> m_shaders; // either one compute shader or vertex/framgment shader pair or raygen/closesthit/miss/... shaders touple

        vk::PipelineCache m_pipelineCache{};
        vk::PipelineLayout m_pipelineLayout{};
        vk::Pipeline m_pipeline{};

        vk::DescriptorPool m_descriptorPool{};
        std::map<uint32_t, uint32_t> m_descriptorSetToIndex;                    // m_descriptorSetToIndex[setId] - mapping of set number to index in the following vectors
        std::vector<Shader::DescriptorSetLayoutData> m_descriptorSetLayoutData; // m_descriptorSetLayoutData[m_descriptorSetToIndex[setId]] - reflected bindings
        std::vector<vk::DescriptorSetLayout> m_descriptorSetLayouts{};          // m_descriptorSetLayouts[m_descriptorSetToIndex[setId]]
        std::vector<std::vector<vk::DescriptorSet>> m_descriptorSets;           // m_descriptorSets[multibufferedId][m_descriptorSetToIndex[setId]] - destroyed implicitly with the descriptor pool

        std::map<uint32_t, std::map<uint32_t, std::shared_ptr<Uniform>>> m_uniforms; // m_uniforms[setId][bindingId]

        vk::QueryPool m_queryPoolTimestamps;
        std::array<uint64_t, 2> m_timestamps = {0, 0};
        bool m_timestampsAvailable = false;
        float m_timeInMs = 0;
        float m_timeInMsAveraged = 0;
        bool m_timeValid = false;

        virtual std::vector<std::shared_ptr<Shader>> createShaders() = 0;

        virtual void createPipeline() = 0;

        virtual uint32_t getPushConstantRangeSize() {
            return 0;
        }

        virtual vk::ShaderStageFlagBits getPushConstantStage() = 0;

        virtual bool getPushConstantRange(vk::PushConstantRange &pushConstantRange) = 0;

        void getDescriptorSets(std::vector<vk::DescriptorSet> &sets) const {
            sets.resize(m_descriptorSets[m_gpuContext->getActiveIndex()].size());
            for (uint32_t i = 0; i < m_descriptorSets[m_gpuContext->getActiveIndex()].size(); i++) {
                sets[i] = m_descriptorSets[m_gpuContext->getActiveIndex()][i];
            }
        }

        void queryTimeFromGPU() {
            if (m_timestampsAvailable) {
                if (m_gpuContext->m_device.getQueryPoolResults(m_queryPoolTimestamps, 0, 2, sizeof(uint64_t) * 2, m_timestamps.data(), sizeof(uint64_t),
                                                               vk::QueryResultFlagBits::e64 | vk::QueryResultFlagBits::eWait) == vk::Result::eSuccess) {
                    const float timeInMs = static_cast<float>(m_timestamps[1] - m_timestamps[0]) * m_gpuContext->getDeviceLimitsTimestampPeriod() / 1000000.0f;
                    m_timeInMs = timeInMs;
                    m_timeInMsAveraged = m_timeValid ? 0.9f * m_timeInMsAveraged + 0.1f * timeInMs : timeInMs;
                    m_timeValid = true;
                } else {
                    m_timeValid = false;
                }
            } else {
                m_timeValid = false;
            }
            m_timestampsAvailable = true;
        }

    private:
        void createPipelineLayout() {
            vk::PipelineLayoutCreateInfo pipelineLayoutInfo{};
            pipelineLayoutInfo.setLayoutCount = m_descriptorSetLayouts.size(); // optional
            pipelineLayoutInfo.pSetLayouts = m_descriptorSetLayouts.data();    // optional

            vk::PushConstantRange pushConstantRange{};
            if (getPushConstantRange(pushConstantRange)) {
                pipelineLayoutInfo.pushConstantRangeCount = 1;               // optional
                pipelineLayoutInfo.pPushConstantRanges = &pushConstantRange; // optional
            } else {
                pipelineLayoutInfo.pushConstantRangeCount = 0;    // optional
                pipelineLayoutInfo.pPushConstantRanges = nullptr; // optional
            }

            if (m_gpuContext->m_device.createPipelineLayout(&pipelineLayoutInfo, nullptr, &m_pipelineLayout) != vk::Result::eSuccess) {
                throw std::runtime_error("Failed to create pipeline layout!");
            }
        }

        void createDescriptorSetLayout() {
            // retrieve information using SPIRV-reflect
            for (const auto &shader: m_shaders) {
                // for every shader
                for (const auto &[fst, snd]: *shader->getDescriptorSetLayoutData()) {
                    // retrieve all sets
                    uint32_t setId = fst;
                    const auto &[set_number, create_info, bindings, bindingToIndex] = snd;

                    if (m_descriptorSetToIndex.contains(setId)) {
                        // merge with existing set
                        const uint32_t index = m_descriptorSetToIndex[setId];
                        auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[index];
                        assert(setId == set_number);
                        bindings.insert(bindings.end(), bindings.begin(), bindings.end());
                        //                        for (auto &binding : layout.bindings) {
                        //
                        //                        }
                    } else {
                        // insert new set
                        const uint32_t index = m_descriptorSetLayoutData.size();
                        m_descriptorSetToIndex[setId] = index;
                        Shader::DescriptorSetLayoutData newLayoutData{.set_number = setId, .bindings = bindings};
                        m_descriptorSetLayoutData.push_back(newLayoutData);
                    }
                }

                for (const auto &uniforms: std::ranges::views::values(*shader->getUniforms())) {
                    for (const auto &uniform: uniforms) {
                        m_uniforms[uniform->getSet()][uniform->getBinding()] = uniform;
                    }
                }
            }

            for (auto &layoutData: m_descriptorSetLayoutData) {
                layoutData.create_info.bindingCount = layoutData.bindings.size();
                layoutData.create_info.pBindings = layoutData.bindings.data();

                layoutData.generateBindingToIndexMap();
            }

            m_descriptorSetLayouts.resize(m_descriptorSetLayoutData.size());
            for (auto &[set_number, create_info, bindings, bindingToIndex]: m_descriptorSetLayoutData) {
                if (const uint32_t index = m_descriptorSetToIndex[set_number]; m_gpuContext->m_device.createDescriptorSetLayout(&create_info, nullptr, &m_descriptorSetLayouts[index]) != vk::Result::eSuccess) {
                    throw std::runtime_error("Failed to create descriptor set layout!");
                }
            }
        }

        void createDescriptorPool() {
            // retrieve information using SPIRV-reflect
            std::map<vk::DescriptorType, uint32_t> descriptorCounts;
            for (const auto &[set_number, create_info, bindings, bindingToIndex]: m_descriptorSetLayoutData) {
                for (const auto &binding: bindings) {
                    if (descriptorCounts.contains(binding.descriptorType)) {
                        descriptorCounts[binding.descriptorType] += binding.descriptorCount;
                    } else {
                        descriptorCounts[binding.descriptorType] = binding.descriptorCount;
                    }
                }
            }

            std::vector<vk::DescriptorPoolSize> poolSizes;
            for (const auto &[descriptorType, count]: descriptorCounts) {
                poolSizes.emplace_back(descriptorType, count * m_gpuContext->getMultiBufferedCount());
            }

            vk::DescriptorPoolCreateInfo poolInfo{};
            poolInfo.poolSizeCount = static_cast<uint32_t>(poolSizes.size());
            poolInfo.pPoolSizes = poolSizes.data();
            poolInfo.maxSets = m_gpuContext->getMultiBufferedCount() * m_descriptorSetLayouts.size();

            if (m_gpuContext->m_device.createDescriptorPool(&poolInfo, nullptr, &m_descriptorPool) != vk::Result::eSuccess) {
                throw std::runtime_error("Failed to create descriptor pool!");
            }
        }

        void createDescriptorSets() {
            // allocate descriptor sets (valid values will be set later)
            m_descriptorSets.resize(m_gpuContext->getMultiBufferedCount());
            for (uint32_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                vk::DescriptorSetAllocateInfo allocInfo{};
                allocInfo.descriptorPool = m_descriptorPool;
                allocInfo.descriptorSetCount = m_descriptorSetLayouts.size();
                allocInfo.pSetLayouts = m_descriptorSetLayouts.data();

                m_descriptorSets[i].resize(m_descriptorSetLayouts.size());
                if (m_gpuContext->m_device.allocateDescriptorSets(&allocInfo, m_descriptorSets[i].data()) != vk::Result::eSuccess) {
                    throw std::runtime_error("Failed to allocate descriptor sets!");
                }
            }
        }

        void createUniforms() {
            for (const auto &uniforms: std::ranges::views::values(m_uniforms)) {
                for (const auto &uniform: std::ranges::views::values(uniforms)) {
                    uniform->create();
                    setUniform(uniform.get());
                }
            }
        }

        void releaseUniforms() {
            for (const auto &uniforms: std::ranges::views::values(m_uniforms)) {
                for (const auto &uniform: std::ranges::views::values(uniforms)) {
                    uniform->release();
                }
            }
        }

        void setUniform(Uniform *uniform) {
            const uint32_t setIdx = m_descriptorSetToIndex[uniform->getSet()];

            auto &[set_number, create_info, bindings, bindingToIndex] = m_descriptorSetLayoutData[setIdx];
            const auto &bindingLayout = bindings[bindingToIndex[uniform->getBinding()]];

            assert(uniform->getSet() == set_number);
            assert(uniform->getBinding() == bindingLayout.binding);

            vk::WriteDescriptorSet writeDescriptorSet{};
            writeDescriptorSet.dstBinding = uniform->getBinding();
            writeDescriptorSet.dstArrayElement = 0;
            writeDescriptorSet.descriptorType = bindingLayout.descriptorType;
            writeDescriptorSet.descriptorCount = 1;

            for (uint32_t i = 0; i < m_gpuContext->getMultiBufferedCount(); i++) {
                vk::DescriptorBufferInfo bufferInfo{};
                uniform->generateBufferInfo(&bufferInfo, i);

                writeDescriptorSet.dstSet = m_descriptorSets[i][setIdx];
                writeDescriptorSet.pBufferInfo = &bufferInfo;
                m_gpuContext->m_device.updateDescriptorSets(1, &writeDescriptorSet, 0, nullptr);
            }
        }

        void createTimestampQueryPool() {
            vk::QueryPoolCreateInfo queryPoolCreateInfo{};
            queryPoolCreateInfo.sType = vk::StructureType::eQueryPoolCreateInfo;
            queryPoolCreateInfo.queryType = vk::QueryType::eTimestamp;
            queryPoolCreateInfo.queryCount = static_cast<uint32_t>(m_timestamps.size());
            if (m_gpuContext->m_device.createQueryPool(&queryPoolCreateInfo, nullptr, &m_queryPoolTimestamps) != vk::Result::eSuccess) {
                throw std::runtime_error("Cannot create query pool!");
            }
        }
    };
} // namespace raven

```

## Source file: ReSTIRSSS/include/restirsss/passes/PassSurface.h

{
  "path": "ReSTIRSSS/include/restirsss/passes/PassSurface.h",
  "blob_sha": "9c4c33793be508bcae864c3a48aef197a643eccd",
  "sha256": "79f8d4dd46e1b7f497d3f88d7ee4eac52c03f1c29906b439d61a184648c79653",
  "lines": 43,
  "bytes": 1689,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 3,
    "shader": 5,
    "rig": 3,
    "face": 3
  },
  "url": "https://github.com/MircoWerner/ReSTIR-SSS/blob/c209f2b72b85cc51aa2577c14534952154ae4313/ReSTIRSSS/include/restirsss/passes/PassSurface.h",
  "function_signature_candidates": []
}

```text

#pragma once

#include "raven/passes/PassShaderCompute.h"
#include "raven/util/Paths.h"

namespace raven {
    class PassSurface final : public PassShaderCompute {
    public:
        explicit PassSurface(GPUContext *gpuContext) : PassShaderCompute(gpuContext, "PassSurface") {
        }

        struct PushConstants {
            glm::ivec2 g_pixels;

            glm::vec3 g_ray_origin;
            glm::vec3 g_ray_left_bottom;
            glm::vec3 g_ray_left_top;
            glm::vec3 g_ray_right_bottom;
            glm::vec3 g_ray_right_top;

            uint32_t g_frame = 0;

            uint32_t g_rng_init;
            uint32_t g_rng_init_offset;
        };

        PushConstants m_pushConstants = {};

    protected:
        std::vector<std::shared_ptr<Shader>> createShaders() override {
            constexpr auto workGroupSize = vk::Extent3D{8, 8, 1};
            auto specializationMapEntries = std::vector{vk::SpecializationMapEntry{0, 0, sizeof(uint32_t)},
                                                        vk::SpecializationMapEntry{1, 4, sizeof(uint32_t)},
                                                        vk::SpecializationMapEntry{2, 8, sizeof(uint32_t)}};
            auto specializationData = std::vector{workGroupSize.width, workGroupSize.height, workGroupSize.depth};
            return {std::make_shared<Shader>(m_gpuContext, Paths::m_resourceDirectoryPath + "/shaders/main/passes", "pass_surface.comp", vk::ShaderStageFlagBits::eCompute, workGroupSize, specializationMapEntries, specializationData)};
        }

        uint32_t getPushConstantRangeSize() override {
            return sizeof(PushConstants);
        }
    };
} // namespace raven

```

## Source file: VkRaven/raven/include/raven/passes/PassShaderCompute.h

{
  "path": "VkRaven/raven/include/raven/passes/PassShaderCompute.h",
  "blob_sha": "5f051cf2ce37864a30c4c8886863956fd14daad7",
  "sha256": "0de08228412ae9cde12197c79c4723e4749566bc81a40ac580b4cce4aa252980",
  "lines": 87,
  "bytes": 4156,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 7,
    "shader": 14
  },
  "url": "https://github.com/MircoWerner/ReSTIR-SSS/blob/c209f2b72b85cc51aa2577c14534952154ae4313/VkRaven/raven/include/raven/passes/PassShaderCompute.h",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "void create() override { PassShader::create(); }"
    },
    {
      "line": 12,
      "text": "void release() override { PassShader::release(); }"
    },
    {
      "line": 14,
      "text": "void setWorkGroupCount(const uint32_t x, const uint32_t y, const uint32_t z) {"
    },
    {
      "line": 18,
      "text": "virtual void setGlobalInvocationSize(const uint32_t width, const uint32_t height, const uint32_t depth) {"
    },
    {
      "line": 30,
      "text": "void execute(const std::function<void(const vk::CommandBuffer &commandBuffer, const vk::PipelineLayout &pipelineLayout)> &recordPushConstants,"
    },
    {
      "line": 59,
      "text": "void createPipeline() override {"
    },
    {
      "line": 73,
      "text": "bool getPushConstantRange(vk::PushConstantRange &pushConstantRange) override {"
    }
  ]
}

```text

#pragma once

#include "PassShader.h"

namespace raven {
    class PassShaderCompute : public PassShader {
    public:
        PassShaderCompute(GPUContext *gpuContext, std::string name) : PassShader(gpuContext, std::move(name)) {}

        void create() override { PassShader::create(); }

        void release() override { PassShader::release(); }

        void setWorkGroupCount(const uint32_t x, const uint32_t y, const uint32_t z) {
            m_workGroupCount = vk::Extent3D{x, y, z};
        }

        virtual void setGlobalInvocationSize(const uint32_t width, const uint32_t height, const uint32_t depth) {
            const vk::Extent3D workGroupSize = m_shaders[0]->getWorkGroupSize();
            m_workGroupCount = getDispatchSize(width, height, depth, workGroupSize);
        }

        static vk::Extent3D getDispatchSize(const uint32_t width, const uint32_t height, const uint32_t depth, const vk::Extent3D workGroupSize) {
            const uint32_t x = (width + workGroupSize.width - 1) / workGroupSize.width;
            const uint32_t y = (height + workGroupSize.height - 1) / workGroupSize.height;
            const uint32_t z = (depth + workGroupSize.depth - 1) / workGroupSize.depth;
            return vk::Extent3D{x, y, z};
        }

        void execute(const std::function<void(const vk::CommandBuffer &commandBuffer, const vk::PipelineLayout &pipelineLayout)> &recordPushConstants,
                     const std::function<void(const vk::CommandBuffer &commandBuffer)> &recordPipelineBarriers,
                     vk::CommandBuffer &commandBuffer) override {
            assert(m_workGroupCount.width >= 1 && m_workGroupCount.height >= 1 && m_workGroupCount.depth >= 1 && "Global invocation size must be set before executing PassShaderCompute!");

            queryTimeFromGPU();

            commandBuffer.resetQueryPool(m_queryPoolTimestamps, 0, static_cast<uint32_t>(m_timestamps.size()));

            recordPushConstants(commandBuffer, m_pipelineLayout);

            commandBuffer.bindPipeline(vk::PipelineBindPoint::eCompute, m_pipeline);

            std::vector<vk::DescriptorSet> descriptorSets;
            getDescriptorSets(descriptorSets);
            commandBuffer.bindDescriptorSets(vk::PipelineBindPoint::eCompute, m_pipelineLayout, 0, descriptorSets.size(), descriptorSets.data(), 0, nullptr);

            commandBuffer.writeTimestamp(vk::PipelineStageFlagBits::eComputeShader, m_queryPoolTimestamps, 0);
            vk::DebugUtilsLabelEXT label{};
            label.pLabelName = m_name.c_str();
            commandBuffer.beginDebugUtilsLabelEXT(&label);
            commandBuffer.dispatch(m_workGroupCount.width, m_workGroupCount.height, m_workGroupCount.depth);
            commandBuffer.endDebugUtilsLabelEXT();
            commandBuffer.writeTimestamp(vk::PipelineStageFlagBits::eComputeShader, m_queryPoolTimestamps, 1);

            recordPipelineBarriers(commandBuffer);
        }

    protected:
        void createPipeline() override {
            assert(m_shaders.size() == 1 && "Only one shader is allowed for PassShaderCompute!");

            const vk::PipelineShaderStageCreateInfo shaderStage = m_shaders[0]->generateShaderStageCreateInfo();

            vk::ComputePipelineCreateInfo pipelineInfo{};
            pipelineInfo.layout = m_pipelineLayout;
            pipelineInfo.stage = shaderStage;

            if (m_gpuContext->m_device.createComputePipelines(m_pipelineCache, 1, &pipelineInfo, nullptr, &m_pipeline) != vk::Result::eSuccess) {
                throw std::runtime_error("Failed to create compute pipeline!");
            }
        }

        bool getPushConstantRange(vk::PushConstantRange &pushConstantRange) override {
            pushConstantRange.stageFlags = getPushConstantStage();
            pushConstantRange.offset = 0;
            pushConstantRange.size = getPushConstantRangeSize();
            return pushConstantRange.size > 0;
        }

        vk::ShaderStageFlagBits getPushConstantStage() override {
            return vk::ShaderStageFlagBits::eCompute;
        }

    private:
        vk::Extent3D m_workGroupCount = {0, 0, 0};
    };
} // namespace raven

```

## Source file: VkRaven/raven/include/raven/passes/PassShaderRayTracing.h

{
  "path": "VkRaven/raven/include/raven/passes/PassShaderRayTracing.h",
  "blob_sha": "02a9d0e7b8790bc515add930aca3beebc2bbff61",
  "sha256": "1239f705746abcf25f2f424ff25692ddae66a03b5aa4ad3b83c7983e2b583d65",
  "lines": 284,
  "bytes": 14537,
  "selection_score": 30,
  "keyword_line_counts": {
    "sss": 17,
    "shader": 80
  },
  "url": "https://github.com/MircoWerner/ReSTIR-SSS/blob/c209f2b72b85cc51aa2577c14534952154ae4313/VkRaven/raven/include/raven/passes/PassShaderRayTracing.h",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "void create() override {"
    },
    {
      "line": 21,
      "text": "void release() override {"
    },
    {
      "line": 28,
      "text": "void setGlobalInvocationSize(const uint32_t width, const uint32_t height, const uint32_t depth) override {"
    },
    {
      "line": 32,
      "text": "void execute(const std::function<void(const vk::CommandBuffer &commandBuffer, const vk::PipelineLayout &pipelineLayout)> &recordPushConstants,"
    },
    {
      "line": 62,
      "text": "void createPipeline() override {"
    },
    {
      "line": 105,
      "text": "bool getPushConstantRange(vk::PushConstantRange &pushConstantRange) override {"
    },
    {
      "line": 132,
      "text": "[[nodiscard]] bool isTriangleHitGroup() const {"
    },
    {
      "line": 136,
      "text": "[[nodiscard]] bool isProceduralHitGroup() const {"
    },
    {
      "line": 177,
      "text": "void createShaderBindingTable() {"
    }
  ]
}

```text

#pragma once

#include "PassShader.h"
#include "PassShaderCompute.h"

namespace raven {
    class PassShaderRayTracing : public PassShaderCompute {
    public:
        PassShaderRayTracing(GPUContext *gpuContext, std::string name) : PassShaderCompute(gpuContext, std::move(name)) {
            vk::PhysicalDeviceProperties2 deviceProperties;
            deviceProperties.pNext = &m_rtPipelineProperties;
            m_gpuContext->m_physicalDevice.getProperties2(&deviceProperties);
        }

        void create() override {
            m_rayGroups = createRayGroups();
            PassShaderCompute::create();
            createShaderBindingTable();
        }

        void release() override {
            RAVEN_BUFFER_RELEASE(m_sbtBufferRayGen);
            RAVEN_BUFFER_RELEASE(m_sbtBufferMiss);
            RAVEN_BUFFER_RELEASE(m_sbtBufferHit);
            PassShaderCompute::release();
        }

        void setGlobalInvocationSize(const uint32_t width, const uint32_t height, const uint32_t depth) override {
            m_invocationSize = vk::Extent3D{width, height, depth};
        }

        void execute(const std::function<void(const vk::CommandBuffer &commandBuffer, const vk::PipelineLayout &pipelineLayout)> &recordPushConstants,
                     const std::function<void(const vk::CommandBuffer &commandBuffer)> &recordPipelineBarriers,
                     vk::CommandBuffer &commandBuffer) override {
            assert(m_invocationSize.width >= 1 && m_invocationSize.height >= 1 && m_invocationSize.depth >= 1 && "Invocation size must be set before executing PassShaderRayTracing!");

            queryTimeFromGPU();

            commandBuffer.resetQueryPool(m_queryPoolTimestamps, 0, static_cast<uint32_t>(m_timestamps.size()));

            recordPushConstants(commandBuffer, m_pipelineLayout);

            commandBuffer.bindPipeline(vk::PipelineBindPoint::eRayTracingKHR, m_pipeline);

            std::vector<vk::DescriptorSet> descriptorSets;
            getDescriptorSets(descriptorSets);
            commandBuffer.bindDescriptorSets(vk::PipelineBindPoint::eRayTracingKHR, m_pipelineLayout, 0, descriptorSets.size(), descriptorSets.data(), 0, nullptr);

            commandBuffer.writeTimestamp(vk::PipelineStageFlagBits::eRayTracingShaderKHR, m_queryPoolTimestamps, 0);
            vk::DebugUtilsLabelEXT label{};
            label.pLabelName = m_name.c_str();
            commandBuffer.beginDebugUtilsLabelEXT(&label);
            commandBuffer.traceRaysKHR(&m_rayGenShaderSBTEntry, &m_missShaderSBTEntry, &m_hitShaderSBTEntry, &m_callableShaderSBTEntry, m_invocationSize.width, m_invocationSize.height,
                                   m_invocationSize.depth);
            commandBuffer.endDebugUtilsLabelEXT();
            commandBuffer.writeTimestamp(vk::PipelineStageFlagBits::eRayTracingShaderKHR, m_queryPoolTimestamps, 1);

            recordPipelineBarriers(commandBuffer);
        }

    protected:
        void createPipeline() override {
            assert(!m_shaders.empty() && "PassShaderRayTracing requires at least one ray generation shader!");

            std::vector<vk::PipelineShaderStageCreateInfo> shaderStages(m_shaders.size());
            for (uint32_t i = 0; i < m_shaders.size(); i++) {
                shaderStages[i] = m_shaders[i]->generateShaderStageCreateInfo();
            }

            uint32_t numRayGenGroups = m_rayGroups.getRayGenGroupCount();
            uint32_t numMissGroups = m_rayGroups.getRayMissGroupCount();
            uint32_t numHitGroups = m_rayGroups.getRayHitGroupCount();
            uint32_t numGroups = numRayGenGroups + numMissGroups + numHitGroups;

            std::vector<vk::RayTracingShaderGroupCreateInfoKHR> groups(numGroups);
            uint32_t groupIndex = 0;
            for (const auto &group: m_rayGroups.rayGenGroups) {
                groups[groupIndex++] = createRayGenGroup(group.rayGenShaderIndex);
            }
            for (const auto &group: m_rayGroups.rayMissGroups) {
                groups[groupIndex++] = createMissGroup(group.missShaderIndex);
            }
            for (const auto &group: m_rayGroups.rayHitGroups) {
                if (group.isTriangleHitGroup()) {
                    groups[groupIndex++] = createTriangleHitGroup(group.hitShaderIndex);
                } else {
                    groups[groupIndex++] = createProceduralHitGroup(group.hitShaderIndex, group.intersectionShaderIndex);
                }
            }

            vk::RayTracingPipelineCreateInfoKHR pipelineInfo{};
            pipelineInfo.sType = vk::StructureType::eRayTracingPipelineCreateInfoKHR;
            pipelineInfo.stageCount = shaderStages.size();
            pipelineInfo.pStages = shaderStages.data();
            pipelineInfo.groupCount = groups.size();
            pipelineInfo.pGroups = groups.data();
            pipelineInfo.maxPipelineRayRecursionDepth = 1;
            pipelineInfo.layout = m_pipelineLayout;

            if (m_gpuContext->m_device.createRayTracingPipelinesKHR(VK_NULL_HANDLE, m_pipelineCache, 1, &pipelineInfo, nullptr, &m_pipeline) != vk::Result::eSuccess) {
                throw std::runtime_error("Failed to create ray tracing pipeline!");
            }
        }

        bool getPushConstantRange(vk::PushConstantRange &pushConstantRange) override {
            pushConstantRange.stageFlags = getPushConstantStage();
            pushConstantRange.offset = 0;
            pushConstantRange.size = getPushConstantRangeSize();
            return pushConstantRange.size > 0;
        }

        vk::ShaderStageFlagBits getPushConstantStage() override {
            return vk::ShaderStageFlagBits::eRaygenKHR;
        }

        enum RayHitGroupType {
            eTriangle,
            eProcedural
        };

        struct RayGenGroup {
            uint32_t rayGenShaderIndex = 0;
        };
        struct RayMissGroup {
            uint32_t missShaderIndex = 0;
        };
        struct RayHitGroup {
            uint32_t hitShaderIndex = 0;
            uint32_t intersectionShaderIndex = 0;
            RayHitGroupType type = eTriangle;

            [[nodiscard]] bool isTriangleHitGroup() const {
                return type == eTriangle;
            }

            [[nodiscard]] bool isProceduralHitGroup() const {
                return type == eProcedural;
            }
        };

        struct RayGroups {
            std::vector<RayGenGroup> rayGenGroups;
            std::vector<RayMissGroup> rayMissGroups;
            std::vector<RayHitGroup> rayHitGroups;

            [[nodiscard]] uint32_t getRayGenGroupCount() const {
                return rayGenGroups.size();
            }

            [[nodiscard]] uint32_t getRayMissGroupCount() const {
                return rayMissGroups.size();
            }

            [[nodiscard]] uint32_t getRayHitGroupCount() const {
                return rayHitGroups.size();
            }
        };

        virtual RayGroups createRayGroups() = 0;

    private:
        vk::Extent3D m_invocationSize = {0, 0, 0};

        vk::PhysicalDeviceRayTracingPipelinePropertiesKHR m_rtPipelineProperties{};

        RayGroups m_rayGroups;

        std::shared_ptr<Buffer> m_sbtBufferRayGen;
        std::shared_ptr<Buffer> m_sbtBufferMiss;
        std::shared_ptr<Buffer> m_sbtBufferHit;

        vk::StridedDeviceAddressRegionKHR m_rayGenShaderSBTEntry{};
        vk::StridedDeviceAddressRegionKHR m_missShaderSBTEntry{};
        vk::StridedDeviceAddressRegionKHR m_hitShaderSBTEntry{};
        vk::StridedDeviceAddressRegionKHR m_callableShaderSBTEntry{};

        void createShaderBindingTable() {
            uint32_t numRayGenGroups = m_rayGroups.getRayGenGroupCount();
            uint32_t numMissGroups = m_rayGroups.getRayMissGroupCount();
            uint32_t numHitGroups = m_rayGroups.getRayHitGroupCount();
            uint32_t numGroups = numRayGenGroups + numMissGroups + numHitGroups;

            const uint32_t handleSizeAligned = alignUp(m_rtPipelineProperties.shaderGroupHandleSize, m_rtPipelineProperties.shaderGroupHandleAlignment);
            const auto groupCount = static_cast<uint32_t>(numGroups);
            const uint32_t sbtSize = groupCount * handleSizeAligned;

            Buffer::BufferSettings bufferSettingsRayGen{.m_sizeBytes = static_cast<uint32_t>(numRayGenGroups * handleSizeAligned),
                                                        .m_bufferUsages = vk::BufferUsageFlagBits::eTransferDst | vk::BufferUsageFlagBits::eShaderDeviceAddress | vk::BufferUsageFlagBits::eShaderBindingTableKHR,
                                                        .m_memoryProperties = vk::MemoryPropertyFlagBits::eDeviceLocal,
                                                        .m_memoryAllocateFlagBits = vk::MemoryAllocateFlagBits::eDeviceAddress,
                                                        .m_name = "sbtRayGen"};
            Buffer::BufferSettings bufferSettingsMiss{.m_sizeBytes = static_cast<uint32_t>(numMissGroups * handleSizeAligned),
                                                      .m_bufferUsages = vk::BufferUsageFlagBits::eTransferDst | vk::BufferUsageFlagBits::eShaderDeviceAddress | vk::BufferUsageFlagBits::eShaderBindingTableKHR,
                                                      .m_memoryProperties = vk::MemoryPropertyFlagBits::eDeviceLocal,
                                                      .m_memoryAllocateFlagBits = vk::MemoryAllocateFlagBits::eDeviceAddress,
                                                      .m_name = "sbtMiss"};
            Buffer::BufferSettings bufferSettingsHit{.m_sizeBytes = static_cast<uint32_t>(numHitGroups * handleSizeAligned),
                                                     .m_bufferUsages = vk::BufferUsageFlagBits::eTransferDst | vk::BufferUsageFlagBits::eShaderDeviceAddress | vk::BufferUsageFlagBits::eShaderBindingTableKHR,
                                                     .m_memoryProperties = vk::MemoryPropertyFlagBits::eDeviceLocal,
                                                     .m_memoryAllocateFlagBits = vk::MemoryAllocateFlagBits::eDeviceAddress,
                                                     .m_name = "sbtHit"};
            m_sbtBufferRayGen = std::make_shared<Buffer>(m_gpuContext, bufferSettingsRayGen);
            m_sbtBufferMiss = std::make_shared<Buffer>(m_gpuContext, bufferSettingsMiss);
            m_sbtBufferHit = std::make_shared<Buffer>(m_gpuContext, bufferSettingsHit);

            std::vector<uint8_t> shaderHandleStorage(sbtSize);
            if (m_gpuContext->m_device.getRayTracingShaderGroupHandlesKHR(m_pipeline, 0, groupCount, sbtSize, shaderHandleStorage.data()) != vk::Result::eSuccess) {
                throw std::runtime_error("Cannot retrieve ray tracing shader group handles.");
            }

            {
                m_sbtBufferRayGen = Buffer::fillDeviceWithStagingBuffer(m_gpuContext, bufferSettingsRayGen, shaderHandleStorage.data());
            }
            {
                m_sbtBufferMiss = Buffer::fillDeviceWithStagingBuffer(m_gpuContext, bufferSettingsMiss, shaderHandleStorage.data() + handleSizeAligned * numRayGenGroups);
            }
            {
                m_sbtBufferHit = Buffer::fillDeviceWithStagingBuffer(m_gpuContext, bufferSettingsHit, shaderHandleStorage.data() + handleSizeAligned * (numRayGenGroups + numMissGroups));
            }

            m_rayGenShaderSBTEntry.deviceAddress = m_sbtBufferRayGen->getDeviceAddress();
            m_rayGenShaderSBTEntry.stride = handleSizeAligned;
            m_rayGenShaderSBTEntry.size = handleSizeAligned;

            m_missShaderSBTEntry.deviceAddress = m_sbtBufferMiss->getDeviceAddress();
            m_missShaderSBTEntry.stride = handleSizeAligned;
            m_missShaderSBTEntry.size = handleSizeAligned * numMissGroups;

            m_hitShaderSBTEntry.deviceAddress = m_sbtBufferHit->getDeviceAddress();
            m_hitShaderSBTEntry.stride = handleSizeAligned;
            m_hitShaderSBTEntry.size = handleSizeAligned * numHitGroups;
        }

        static vk::RayTracingShaderGroupCreateInfoKHR createRayGenGroup(uint32_t rayGenShaderIndex) {
            return createGeneralGroup(rayGenShaderIndex);
        }
        static vk::RayTracingShaderGroupCreateInfoKHR createMissGroup(uint32_t missShaderIndex) {
            return createGeneralGroup(missShaderIndex);
        }
        static vk::RayTracingShaderGroupCreateInfoKHR createTriangleHitGroup(uint32_t hitShaderIndex) {
            vk::RayTracingShaderGroupCreateInfoKHR group{};
            group.sType = vk::StructureType::eRayTracingShaderGroupCreateInfoKHR;
            group.type = vk::RayTracingShaderGroupTypeKHR::eTrianglesHitGroup;
            group.generalShader = VK_SHADER_UNUSED_KHR;
            group.closestHitShader = hitShaderIndex;
            group.anyHitShader = VK_SHADER_UNUSED_KHR;
            group.intersectionShader = VK_SHADER_UNUSED_KHR;
            return group;
        }
        static vk::RayTracingShaderGroupCreateInfoKHR createProceduralHitGroup(uint32_t hitShaderIndex, uint32_t intersectionShaderIndex) {
            vk::RayTracingShaderGroupCreateInfoKHR group{};
            group.sType = vk::StructureType::eRayTracingShaderGroupCreateInfoKHR;
            group.type = vk::RayTracingShaderGroupTypeKHR::eProceduralHitGroup;
            group.generalShader = VK_SHADER_UNUSED_KHR;
            group.closestHitShader = hitShaderIndex;
            group.anyHitShader = VK_SHADER_UNUSED_KHR;
            group.intersectionShader = intersectionShaderIndex;
            return group;
        }
        static vk::RayTracingShaderGroupCreateInfoKHR createGeneralGroup(uint32_t generalShaderIndex) {
            vk::RayTracingShaderGroupCreateInfoKHR group{};
            group.sType = vk::StructureType::eRayTracingShaderGroupCreateInfoKHR;
            group.type = vk::RayTracingShaderGroupTypeKHR::eGeneral;
            group.generalShader = generalShaderIndex;
            group.closestHitShader = VK_SHADER_UNUSED_KHR;
            group.anyHitShader = VK_SHADER_UNUSED_KHR;
            group.intersectionShader = VK_SHADER_UNUSED_KHR;
            return group;
        }

        static uint32_t alignUp(uint32_t toAlign, uint32_t multiple) {
            if (multiple == 0) {
                return toAlign;
            }

            uint32_t remainder = toAlign % multiple;
            if (remainder == 0) {
                return toAlign;
            }

            return toAlign + multiple - remainder;
        }
    };
} // namespace raven

```
