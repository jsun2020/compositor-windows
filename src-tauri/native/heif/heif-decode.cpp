// Portable HEVC/HEIF decoder. The LGPL libraries remain replaceable shared libraries.
// HIF1: magic, width, height, storage bits (16), ICC bytes, source bits, reserved (0),
// then optional source ICC and straight RGBA16 little-endian pixels. Without ICC,
// libheif converts the image's NCLX color space to sRGB before returning pixels.
// libheif 1.23.6 propagates its producer macro through the imported CMake target.
// This executable consumes the shared library and must use DLL imports.
#ifdef LIBHEIF_EXPORTS
#undef LIBHEIF_EXPORTS
#endif
#include <libheif/heif.h>
#include <algorithm>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <new>
#include <vector>

namespace fs = std::filesystem;
static void check(heif_error e) {
    if (e.code != heif_error_Ok) throw std::runtime_error(e.message ? e.message : "HEIF decode failed");
}
static void put32(std::ostream& out, uint32_t n) {
    const char b[4] = {char(n), char(n >> 8), char(n >> 16), char(n >> 24)}; out.write(b, 4);
}
static void dimensions(int w, int h, uint64_t budget) {
    if (w <= 0 || h <= 0 || w > 30000 || h > 30000 || uint64_t(w) * uint64_t(h) > budget)
        throw std::runtime_error("HEIF image exceeds the remaining document pixel budget or 30,000 pixel side limit");
}
static int decode(const fs::path& input, const fs::path& output, uint64_t budget) {
    if (!budget || budget > 100000000) throw std::runtime_error("Invalid HEIF pixel budget");
    if (!fs::is_regular_file(input) || fs::file_size(input) > 512ull * 1024 * 1024)
        throw std::runtime_error("HEIF input must be a file under 512 MiB");
    if (fs::exists(output)) throw std::runtime_error("Refusing to overwrite HEIF output");
    std::ifstream file(input, std::ios::binary);
    std::vector<uint8_t> bytes(static_cast<size_t>(fs::file_size(input)));
    if (!file.read(reinterpret_cast<char*>(bytes.data()), std::streamsize(bytes.size())))
        throw std::runtime_error("Cannot read HEIF input");
    check(heif_init(nullptr));
    struct Library { ~Library() { heif_deinit(); } } library;
    std::unique_ptr<heif_context, decltype(&heif_context_free)> context(heif_context_alloc(), heif_context_free);
    if (!context) throw std::bad_alloc();
    auto limits = *heif_context_get_security_limits(context.get());
    limits.max_image_size_pixels = budget;
    limits.max_color_profile_size = 16 * 1024 * 1024;
    limits.max_memory_block_size = 1024ull * 1024 * 1024;
    limits.max_total_memory = 1536ull * 1024 * 1024;
    limits.max_items = std::min(limits.max_items, uint32_t(10000));
    check(heif_context_set_security_limits(context.get(), &limits));
    heif_context_set_max_decoding_threads(context.get(), 1);
    check(heif_context_read_from_memory_without_copy(context.get(), bytes.data(), bytes.size(), nullptr));
    heif_image_handle* raw_handle = nullptr;
    check(heif_context_get_primary_image_handle(context.get(), &raw_handle));
    std::unique_ptr<heif_image_handle, decltype(&heif_image_handle_release)> handle(raw_handle, heif_image_handle_release);
    dimensions(heif_image_handle_get_width(handle.get()), heif_image_handle_get_height(handle.get()), budget);
    const int source_bits = heif_image_handle_get_luma_bits_per_pixel(handle.get());
    if (source_bits < 8 || source_bits > 16) throw std::runtime_error("Unsupported HEIF bit depth");
    size_t profile_size = heif_image_handle_get_raw_color_profile_size(handle.get());
    if (profile_size > 16 * 1024 * 1024) throw std::runtime_error("HEIF ICC profile exceeds 16 MiB");
    std::vector<uint8_t> profile(profile_size);
    if (profile_size) check(heif_image_handle_get_raw_color_profile(handle.get(), profile.data()));
    std::unique_ptr<heif_decoding_options, decltype(&heif_decoding_options_free)> options(heif_decoding_options_alloc(), heif_decoding_options_free);
    if (!options) throw std::bad_alloc();
    options->ignore_transformations = 0;
    options->strict_decoding = 1;
    options->convert_hdr_to_8bit = 0;
    options->color_conversion_options.preferred_chroma_upsampling_algorithm = heif_chroma_upsampling_bilinear;
    options->color_conversion_options.only_use_preferred_chroma_algorithm = 1;
    options->num_codec_threads = 1;
    options->num_library_threads = 1;
    // ICC is applied once by the host's 16-bit color transform; do not also apply NCLX gamut conversion.
    options->output_image_nclx_profile_passthrough = profile_size ? 1 : 0;
    const auto chroma = source_bits > 8 ? heif_chroma_interleaved_RRGGBBAA_LE : heif_chroma_interleaved_RGBA;
    heif_image* raw_image = nullptr;
    check(heif_decode_image(handle.get(), &raw_image, heif_colorspace_RGB, chroma, options.get()));
    std::unique_ptr<heif_image, decltype(&heif_image_release)> image(raw_image, heif_image_release);
    const int w = heif_image_get_width(image.get(), heif_channel_interleaved);
    const int h = heif_image_get_height(image.get(), heif_channel_interleaved);
    dimensions(w, h, budget);
    const int bits = heif_image_get_bits_per_pixel_range(image.get(), heif_channel_interleaved);
    if (bits < 8 || bits > 16 || (source_bits > 8 && bits <= 8)) throw std::runtime_error("HEIF decoder lost source precision");
    const int sample_bytes = source_bits > 8 ? 2 : 1;
    int stride = 0;
    const auto* pixels = heif_image_get_plane_readonly(image.get(), heif_channel_interleaved, &stride);
    if (!pixels || stride < w * 4 * sample_bytes) throw std::runtime_error("Invalid HEIF pixel stride");
    const uint32_t maximum = (uint32_t(1) << bits) - 1;
    const bool premultiplied = heif_image_is_premultiplied_alpha(image.get()) != 0;
    std::ofstream out(output, std::ios::binary);
    if (!out) throw std::runtime_error("Cannot create HEIF output");
    out.write("HIF1", 4); put32(out, w); put32(out, h); put32(out, 16);
    put32(out, uint32_t(profile_size)); put32(out, uint32_t(source_bits)); put32(out, 0);
    out.write(reinterpret_cast<const char*>(profile.data()), std::streamsize(profile_size));
    std::vector<uint8_t> row(size_t(w) * 8);
    for (int y = 0; y < h; ++y) {
        const auto* from = pixels + size_t(y) * size_t(stride);
        for (int x = 0; x < w; ++x) {
            uint32_t rgba[4];
            for (int c = 0; c < 4; ++c) {
                size_t at = (size_t(x) * 4 + c) * sample_bytes;
                uint32_t value = from[at];
                if (sample_bytes == 2) value |= uint32_t(from[at + 1]) << 8;
                if (value > maximum) throw std::runtime_error("HEIF sample exceeds declared precision");
                rgba[c] = (value * 65535u + maximum / 2) / maximum;
            }
            if (premultiplied) for (int c = 0; c < 3; ++c)
                rgba[c] = rgba[3] ? std::min(65535u, (rgba[c] * 65535u + rgba[3] / 2) / rgba[3]) : 0;
            for (int c = 0; c < 4; ++c) { size_t at = size_t(x) * 8 + c * 2; row[at] = uint8_t(rgba[c]); row[at + 1] = uint8_t(rgba[c] >> 8); }
        }
        out.write(reinterpret_cast<const char*>(row.data()), std::streamsize(row.size()));
        if (!out) throw std::runtime_error("Cannot write HEIF pixels");
    }
    out.close(); if (!out) throw std::runtime_error("Cannot finish HEIF output");
    return 0;
}
#ifdef _WIN32
int wmain(int argc, wchar_t** argv) {
#else
int main(int argc, char** argv) {
#endif
    try {
        if (argc != 4) throw std::runtime_error("Usage: CompositorHeif input.heic output.hif1 remaining-pixels");
        const auto budget_text = fs::path(argv[3]).string();
        size_t end = 0; auto budget = std::stoull(budget_text, &end);
        if (end != budget_text.size()) throw std::runtime_error("Invalid pixel budget");
        return decode(fs::path(argv[1]), fs::path(argv[2]), budget);
    } catch (const std::exception& e) { std::cerr << e.what() << '\n'; return 1; }
}
