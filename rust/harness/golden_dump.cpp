/*
TGSpeechBox — Golden reference dump for the Rust port.
Copyright 2025-2026 Tamas Geczy.
Licensed under the MIT License. See LICENSE for details.

Runs a fixed multilingual corpus through the C++ frontend + DSP and writes,
per case, the exact frame track the frontend emitted and the exact PCM the
DSP produced. The Rust port consumes these files:

  - Phase 1 (DSP):     feed <case>.frames into the Rust engine, compare the
                       rendered audio against <case>.pcm.
  - Phase 2 (frontend): feed the same IPA/speed into the Rust frontend,
                       compare the emitted track against <case>.frames.

Usage (from the repository root):
  golden_dump.exe <packDir> <outDir>

<packDir> must be the directory that CONTAINS the packs/ folder (the
repository root). Output files are written into <outDir>, which is created
if missing. A manifest.tsv lists every case.

The corpus is fixed in code on purpose: goldens are only useful if they
never drift. Regenerate all files in one go after an intentional C++
change, and say so in the commit.
*/

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "nvspFrontend.h"
#include "speechPlayer.h"

namespace {

// ----------------------------------------------------------------------------
// Frame track records
// ----------------------------------------------------------------------------

struct FrameRec {
  bool hasFrame = false;
  bool hasEx = false;
  double durationMs = 0.0;
  double fadeMs = 0.0;
  std::int32_t userIndex = 0;
  speechPlayer_frame_t frame{};
  speechPlayer_frameEx_t ex{};
};

struct Case {
  const char* name;
  const char* lang;
  double speed;
  int sampleRate;
  const char* ipa;
  double timeStretch;   // 1.0 = off
  bool setVoicingTone;  // true = apply SPEECHPLAYER_VOICINGTONE_DEFAULTS
};

// Fixed corpus. Speeds 0.5 / 1.0 / 1.6 / 2.0 / 3.0 are the rates AGENTS.md
// requires for anything timing-related; sample rates cover the DSP's
// Nyquist-dependent branches (11025 has the near-Nyquist frication path,
// 44100 the F7/F8 cascade).
const Case kCases[] = {
    {"en-us_base_11025", "en-us", 1.0, 11025,
     "ðə kwˈɪk bɹˈa͡ʊn fˈɑːks d͡ʒˈʌmps ˌo͡ʊvɚ ðə lˈe͡ɪzi dˈɔːɡ",
     1.0, false},
    {"en-us_slow_16000", "en-us", 0.5, 16000,
     "ðə kwˈɪk bɹˈa͡ʊn fˈɑːks d͡ʒˈʌmps ˌo͡ʊvɚ ðə lˈe͡ɪzi dˈɔːɡ",
     1.0, false},
    {"en-us_rate16_22050", "en-us", 1.6, 22050,
     "ðə kwˈɪk bɹˈa͡ʊn fˈɑːks d͡ʒˈʌmps ˌo͡ʊvɚ ðə lˈe͡ɪzi dˈɔːɡ",
     1.0, false},
    {"en-us_2x_44100", "en-us", 2.0, 44100,
     "ðə kwˈɪk bɹˈa͡ʊn fˈɑːks d͡ʒˈʌmps ˌo͡ʊvɚ ðə lˈe͡ɪzi dˈɔːɡ",
     1.0, false},
    {"en-us_3x_16000", "en-us", 3.0, 16000,
     "ðə kwˈɪk bɹˈa͡ʊn fˈɑːks d͡ʒˈʌmps ˌo͡ʊvɚ ðə lˈe͡ɪzi dˈɔːɡ",
     1.0, false},
    {"en-us_stretch2_16000", "en-us", 2.0, 16000,
     "ðə kwˈɪk bɹˈa͡ʊn fˈɑːks d͡ʒˈʌmps ˌo͡ʊvɚ ðə lˈe͡ɪzi dˈɔːɡ",
     2.0, false},
    {"en-us_tone_16000", "en-us", 1.0, 16000,
     "ðə kwˈɪk bɹˈa͡ʊn fˈɑːks d͡ʒˈʌmps ˌo͡ʊvɚ ðə lˈe͡ɪzi dˈɔːɡ",
     1.0, true},
    {"hu_base_16000", "hu", 1.0, 16000,
     "ɒ ɟˈorʃ bˈɒrnɒ rˈoːkɒ ˈaːtuɡorjɒ ɒ lˈuʃtɒ kˈucaːt",
     1.0, false},
    {"hu_affricates_22050", "hu", 1.6, 22050,
     "bˈølt͡ʃʃeːɡ ˈaːt͡ʃʃaːɡ ɲˈolt͡ssor kˈilɛnt͡ssaːz",
     1.0, false},
    {"es_taps_16000", "es", 1.0, 16000,
     "bˈeɾðe tˈaɾðe kˈaɾne ˈaɾma aɾt͡ʃˈiβo maɾt͡ʃˈaɾ pˈaðɾe kwˈaðɾo laðɾˈon dɾˈama ˈalɣo",
     1.0, false},
    {"es_3x_22050", "es", 3.0, 22050,
     "mˈut͡ʃo ˈaɾβol rˈeɣla iɣlˈesja sˈiɣlo",
     1.0, false},
    {"pt_oi_16000", "pt", 1.0, 16000,
     "ˈo͡ɪ fˈo͡ɪ bˈo͡ɪ dˈo͡ɪs pˈo͡ɪs",
     1.0, false},
};

// Same defaults tgsbRender uses: pitch slider 50 -> 110 Hz, inflection 0.5.
constexpr double kBasePitchHz = 110.0;
constexpr double kInflection = 0.5;
constexpr unsigned int kChunk = 2048;

// ----------------------------------------------------------------------------
// Collection + queue callback (mirrors tgsbRender's onFrontendFrameEx)
// ----------------------------------------------------------------------------

struct Ctx {
  speechPlayer_handle_t player = nullptr;
  int sampleRate = 16000;
  std::vector<FrameRec>* recs = nullptr;
};

unsigned int msToSamples(double ms, int sampleRate) {
  if (ms <= 0.0) return 0;
  const double s = (ms * static_cast<double>(sampleRate)) / 1000.0;
  if (s <= 0.0) return 0;
  return static_cast<unsigned int>(s + 0.5);
}

void onFrameEx(void* userData,
               const nvspFrontend_Frame* frameOrNull,
               const nvspFrontend_FrameEx* frameExOrNull,
               double durationMs,
               double fadeMs,
               int userIndex) {
  auto* ctx = static_cast<Ctx*>(userData);
  if (!ctx || !ctx->player || !ctx->recs) return;

  static_assert(sizeof(nvspFrontend_Frame) == sizeof(speechPlayer_frame_t),
                "Frame ABI mismatch");
  static_assert(sizeof(nvspFrontend_FrameEx) == sizeof(speechPlayer_frameEx_t),
                "FrameEx ABI mismatch");

  FrameRec rec;
  rec.durationMs = durationMs;
  rec.fadeMs = fadeMs;
  rec.userIndex = userIndex;

  const unsigned int minSamples = msToSamples(durationMs, ctx->sampleRate);
  const unsigned int fadeSamples = msToSamples(fadeMs, ctx->sampleRate);

  if (frameOrNull) {
    rec.hasFrame = true;
    std::memcpy(&rec.frame, frameOrNull, sizeof(rec.frame));
    if (frameExOrNull) {
      rec.hasEx = true;
      std::memcpy(&rec.ex, frameExOrNull, sizeof(rec.ex));
      speechPlayer_queueFrameEx(
          ctx->player, &rec.frame, &rec.ex,
          static_cast<unsigned int>(sizeof(speechPlayer_frameEx_t)),
          minSamples, fadeSamples, userIndex, false);
    } else {
      speechPlayer_queueFrame(ctx->player, &rec.frame, minSamples,
                              fadeSamples, userIndex, false);
    }
  } else {
    speechPlayer_queueFrame(ctx->player, nullptr, minSamples, fadeSamples,
                            userIndex, false);
  }
  ctx->recs->push_back(rec);
}

// ----------------------------------------------------------------------------
// File writers
// ----------------------------------------------------------------------------

bool writeFrames(const std::string& path, const std::vector<FrameRec>& recs) {
  std::FILE* f = std::fopen(path.c_str(), "wb");
  if (!f) return false;
  const char magic[8] = {'T', 'G', 'S', 'B', 'F', '0', '1', '\0'};
  const std::uint32_t count = static_cast<std::uint32_t>(recs.size());
  const std::uint32_t frameParams = 47;
  const std::uint32_t exParams = 32;
  std::fwrite(magic, 1, 8, f);
  std::fwrite(&count, 4, 1, f);
  std::fwrite(&frameParams, 4, 1, f);
  std::fwrite(&exParams, 4, 1, f);
  for (const auto& r : recs) {
    const std::uint8_t flags =
        (r.hasFrame ? 1u : 0u) | (r.hasEx ? 2u : 0u);
    std::fwrite(&flags, 1, 1, f);
    std::fwrite(&r.durationMs, 8, 1, f);
    std::fwrite(&r.fadeMs, 8, 1, f);
    std::fwrite(&r.userIndex, 4, 1, f);
    if (r.hasFrame) std::fwrite(&r.frame, sizeof(r.frame), 1, f);
    if (r.hasEx) std::fwrite(&r.ex, sizeof(r.ex), 1, f);
  }
  std::fclose(f);
  return true;
}

bool writePcm(const std::string& path, const std::vector<sample>& pcm) {
  std::FILE* f = std::fopen(path.c_str(), "wb");
  if (!f) return false;
  if (!pcm.empty())
    std::fwrite(pcm.data(), sizeof(sample), pcm.size(), f);
  std::fclose(f);
  return true;
}

// ----------------------------------------------------------------------------
// One case
// ----------------------------------------------------------------------------

int runCase(const Case& c, const char* packDir, const std::string& outDir,
            std::FILE* manifest) {
  nvspFrontend_handle_t fe = nvspFrontend_create(packDir);
  if (!fe) {
    std::fprintf(stderr, "[%s] nvspFrontend_create failed\n", c.name);
    return 1;
  }
  if (!nvspFrontend_setLanguage(fe, c.lang)) {
    std::fprintf(stderr, "[%s] setLanguage(%s) failed: %s\n", c.name, c.lang,
                 nvspFrontend_getLastError(fe));
    nvspFrontend_destroy(fe);
    return 1;
  }

  speechPlayer_handle_t player = speechPlayer_initialize(c.sampleRate);
  if (!player) {
    std::fprintf(stderr, "[%s] speechPlayer_initialize failed\n", c.name);
    nvspFrontend_destroy(fe);
    return 1;
  }
  if (c.timeStretch != 1.0)
    speechPlayer_setTimeStretch(player, c.timeStretch);
  if (c.setVoicingTone) {
    const speechPlayer_voicingTone_t tone = SPEECHPLAYER_VOICINGTONE_DEFAULTS;
    speechPlayer_setVoicingTone(player, &tone);
  }

  std::vector<FrameRec> recs;
  Ctx ctx;
  ctx.player = player;
  ctx.sampleRate = c.sampleRate;
  ctx.recs = &recs;

  nvspFrontend_beginStream(fe);
  char clause[2] = {'.', '\0'};
  const int ok = nvspFrontend_queueIPA_Ex(
      fe, c.ipa, c.speed, kBasePitchHz, kInflection, clause, 0, &onFrameEx,
      &ctx);
  if (!ok) {
    std::fprintf(stderr, "[%s] queueIPA_Ex failed: %s\n", c.name,
                 nvspFrontend_getLastError(fe));
    speechPlayer_terminate(player);
    nvspFrontend_destroy(fe);
    return 1;
  }

  std::vector<sample> pcm(kChunk);
  std::vector<sample> all;
  while (true) {
    const int n = speechPlayer_synthesize(
        player, static_cast<unsigned int>(pcm.size()), pcm.data());
    if (n <= 0) break;
    all.insert(all.end(), pcm.begin(), pcm.begin() + n);
    if (n < static_cast<int>(pcm.size())) break;
  }

  const std::string framesPath = outDir + "/" + c.name + ".frames";
  const std::string pcmPath = outDir + "/" + c.name + ".pcm";
  if (!writeFrames(framesPath, recs)) {
    std::fprintf(stderr, "[%s] cannot write %s\n", c.name, framesPath.c_str());
    speechPlayer_terminate(player);
    nvspFrontend_destroy(fe);
    return 1;
  }
  if (!writePcm(pcmPath, all)) {
    std::fprintf(stderr, "[%s] cannot write %s\n", c.name, pcmPath.c_str());
    speechPlayer_terminate(player);
    nvspFrontend_destroy(fe);
    return 1;
  }

  std::fprintf(manifest, "%s\t%s\t%.2f\t%d\t%.1f\t%d\t%zu\t%zu\n",
               c.name, c.lang, c.speed, c.sampleRate, c.timeStretch,
               c.setVoicingTone ? 1 : 0, recs.size(), all.size());
  std::fprintf(stderr, "[%s] %zu frames, %zu samples\n", c.name, recs.size(),
               all.size());

  speechPlayer_terminate(player);
  nvspFrontend_destroy(fe);
  return 0;
}

}  // namespace

int main(int argc, char** argv) {
  if (argc != 3) {
    std::fprintf(stderr, "usage: %s <packDir> <outDir>\n", argv[0]);
    return 2;
  }
  const char* packDir = argv[1];
  const std::string outDir = argv[2];

  // Create the output directory (portable-enough: the harness runs from the
  // repo on Windows and Linux CI alike).
#if defined(_WIN32)
  std::string mkdirCmd = "if not exist \"" + outDir + "\" mkdir \"" + outDir + "\"";
  if (std::system(mkdirCmd.c_str()) != 0) {
    // Fall through: it may already exist.
  }
#else
  std::string mkdirCmd = "mkdir -p '" + outDir + "'";
  if (std::system(mkdirCmd.c_str()) != 0) {
    // Fall through: it may already exist.
  }
#endif

  const std::string manifestPath = outDir + "/manifest.tsv";
  std::FILE* manifest = std::fopen(manifestPath.c_str(), "w");
  if (!manifest) {
    std::fprintf(stderr, "cannot write %s\n", manifestPath.c_str());
    return 2;
  }
  std::fprintf(manifest,
               "name\tlang\tspeed\tsample_rate\ttime_stretch\tvoicing_tone\t"
               "frames\tpcm_samples\n");

  int failures = 0;
  for (const auto& c : kCases) failures += runCase(c, packDir, outDir, manifest);
  std::fclose(manifest);

  if (failures) {
    std::fprintf(stderr, "%d/%zu cases FAILED\n", failures,
                 sizeof(kCases) / sizeof(kCases[0]));
    return 1;
  }
  std::fprintf(stderr, "all %zu cases written to %s\n",
               sizeof(kCases) / sizeof(kCases[0]), outDir.c_str());
  return 0;
}
