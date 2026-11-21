/*
TGSpeechBox — ABI dump for the Rust port.
Copyright 2025-2026 Tamas Geczy.
Licensed under the MIT License. See LICENSE for details.

Prints sizeof / offsetof / default-bit-pattern lines for the POD structs in
src/frame.h, src/sample.h and src/voicingTone.h. The Rust test suite in
rust/crates/tgsb-frame/tests/abi.rs compares its structs against this dump.

Regenerate rust/harness/abi_reference.txt with build_abi_dump.ps1 whenever
those headers change.
*/

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>

#include "src/frame.h"
#include "src/sample.h"
#include "src/voicingTone.h"

static void printBits(const char* st, const char* field, double v) {
  std::uint64_t b;
  std::memcpy(&b, &v, sizeof(b));
  std::printf("bits %s %s 0x%016llX\n", st, field,
              static_cast<unsigned long long>(b));
}

static void printU32(const char* st, const char* field, std::uint32_t v) {
  std::printf("u32 %s %s 0x%08X\n", st, field, static_cast<unsigned>(v));
}

#define SZ(T) std::printf("sizeof %s %zu\n", #T, sizeof(T))
#define OFF(T, m) std::printf("offset %s %s %zu\n", #T, #m, offsetof(T, m))

int main() {
  SZ(speechPlayer_frame_t);
  SZ(speechPlayer_frameEx_t);
  SZ(speechPlayer_voicingTone_t);
  SZ(sample);

  std::printf("scalar speechPlayer_frame_numParams %d\n",
              speechPlayer_frame_numParams);
  std::printf("scalar speechPlayer_frameEx_numParams %d\n",
              speechPlayer_frameEx_numParams);
  std::printf("scalar SPEECHPLAYER_DSP_VERSION %u\n", SPEECHPLAYER_DSP_VERSION);
  std::printf("scalar SPEECHPLAYER_VOICINGTONE_MAGIC %u\n",
              SPEECHPLAYER_VOICINGTONE_MAGIC);
  std::printf("scalar SPEECHPLAYER_VOICINGTONE_VERSION %u\n",
              SPEECHPLAYER_VOICINGTONE_VERSION);

  OFF(speechPlayer_frame_t, voicePitch);
  OFF(speechPlayer_frame_t, vibratoPitchOffset);
  OFF(speechPlayer_frame_t, vibratoSpeed);
  OFF(speechPlayer_frame_t, voiceTurbulenceAmplitude);
  OFF(speechPlayer_frame_t, glottalOpenQuotient);
  OFF(speechPlayer_frame_t, voiceAmplitude);
  OFF(speechPlayer_frame_t, aspirationAmplitude);
  OFF(speechPlayer_frame_t, cf1);
  OFF(speechPlayer_frame_t, cf2);
  OFF(speechPlayer_frame_t, cf3);
  OFF(speechPlayer_frame_t, cf4);
  OFF(speechPlayer_frame_t, cf5);
  OFF(speechPlayer_frame_t, cf6);
  OFF(speechPlayer_frame_t, cfN0);
  OFF(speechPlayer_frame_t, cfNP);
  OFF(speechPlayer_frame_t, cb1);
  OFF(speechPlayer_frame_t, cb2);
  OFF(speechPlayer_frame_t, cb3);
  OFF(speechPlayer_frame_t, cb4);
  OFF(speechPlayer_frame_t, cb5);
  OFF(speechPlayer_frame_t, cb6);
  OFF(speechPlayer_frame_t, cbN0);
  OFF(speechPlayer_frame_t, cbNP);
  OFF(speechPlayer_frame_t, caNP);
  OFF(speechPlayer_frame_t, fricationAmplitude);
  OFF(speechPlayer_frame_t, pf1);
  OFF(speechPlayer_frame_t, pf2);
  OFF(speechPlayer_frame_t, pf3);
  OFF(speechPlayer_frame_t, pf4);
  OFF(speechPlayer_frame_t, pf5);
  OFF(speechPlayer_frame_t, pf6);
  OFF(speechPlayer_frame_t, pb1);
  OFF(speechPlayer_frame_t, pb2);
  OFF(speechPlayer_frame_t, pb3);
  OFF(speechPlayer_frame_t, pb4);
  OFF(speechPlayer_frame_t, pb5);
  OFF(speechPlayer_frame_t, pb6);
  OFF(speechPlayer_frame_t, pa1);
  OFF(speechPlayer_frame_t, pa2);
  OFF(speechPlayer_frame_t, pa3);
  OFF(speechPlayer_frame_t, pa4);
  OFF(speechPlayer_frame_t, pa5);
  OFF(speechPlayer_frame_t, pa6);
  OFF(speechPlayer_frame_t, parallelBypass);
  OFF(speechPlayer_frame_t, preFormantGain);
  OFF(speechPlayer_frame_t, outputGain);
  OFF(speechPlayer_frame_t, endVoicePitch);

  OFF(speechPlayer_frameEx_t, creakiness);
  OFF(speechPlayer_frameEx_t, breathiness);
  OFF(speechPlayer_frameEx_t, jitter);
  OFF(speechPlayer_frameEx_t, shimmer);
  OFF(speechPlayer_frameEx_t, sharpness);
  OFF(speechPlayer_frameEx_t, endCf1);
  OFF(speechPlayer_frameEx_t, endCf2);
  OFF(speechPlayer_frameEx_t, endCf3);
  OFF(speechPlayer_frameEx_t, endPf1);
  OFF(speechPlayer_frameEx_t, endPf2);
  OFF(speechPlayer_frameEx_t, endPf3);
  OFF(speechPlayer_frameEx_t, fujisakiEnabled);
  OFF(speechPlayer_frameEx_t, fujisakiReset);
  OFF(speechPlayer_frameEx_t, fujisakiPhraseAmp);
  OFF(speechPlayer_frameEx_t, fujisakiPhraseLen);
  OFF(speechPlayer_frameEx_t, fujisakiAccentAmp);
  OFF(speechPlayer_frameEx_t, fujisakiAccentDur);
  OFF(speechPlayer_frameEx_t, fujisakiAccentLen);
  OFF(speechPlayer_frameEx_t, transF1Scale);
  OFF(speechPlayer_frameEx_t, transF2Scale);
  OFF(speechPlayer_frameEx_t, transF3Scale);
  OFF(speechPlayer_frameEx_t, transNasalScale);
  OFF(speechPlayer_frameEx_t, transAmplitudeMode);
  OFF(speechPlayer_frameEx_t, cf7);
  OFF(speechPlayer_frameEx_t, cb7);
  OFF(speechPlayer_frameEx_t, cf8);
  OFF(speechPlayer_frameEx_t, cb8);
  OFF(speechPlayer_frameEx_t, transSourceHoldRatio);
  OFF(speechPlayer_frameEx_t, transVoicingHoldRatio);
  OFF(speechPlayer_frameEx_t, fricationTiltDb);
  OFF(speechPlayer_frameEx_t, endVoiceAmplitude);
  OFF(speechPlayer_frameEx_t, amplitudeOnsetMs);

  OFF(speechPlayer_voicingTone_t, magic);
  OFF(speechPlayer_voicingTone_t, structSize);
  OFF(speechPlayer_voicingTone_t, structVersion);
  OFF(speechPlayer_voicingTone_t, dspVersion);
  OFF(speechPlayer_voicingTone_t, voicingPeakPos);
  OFF(speechPlayer_voicingTone_t, voicedPreEmphA);
  OFF(speechPlayer_voicingTone_t, voicedPreEmphMix);
  OFF(speechPlayer_voicingTone_t, highShelfGainDb);
  OFF(speechPlayer_voicingTone_t, highShelfFcHz);
  OFF(speechPlayer_voicingTone_t, highShelfQ);
  OFF(speechPlayer_voicingTone_t, voicedTiltDbPerOct);
  OFF(speechPlayer_voicingTone_t, noiseGlottalModDepth);
  OFF(speechPlayer_voicingTone_t, pitchSyncF1DeltaHz);
  OFF(speechPlayer_voicingTone_t, pitchSyncB1DeltaHz);
  OFF(speechPlayer_voicingTone_t, speedQuotient);
  OFF(speechPlayer_voicingTone_t, aspirationTiltDbPerOct);
  OFF(speechPlayer_voicingTone_t, cascadeBwScale);
  OFF(speechPlayer_voicingTone_t, tremorDepth);
  OFF(speechPlayer_voicingTone_t, nasalBwScale);
  OFF(speechPlayer_voicingTone_t, f4FreqScale);
  OFF(speechPlayer_voicingTone_t, nasalGainScale);
  OFF(speechPlayer_voicingTone_t, chorusDepth);
  OFF(speechPlayer_voicingTone_t, chorusDetuneHz);

  OFF(sample, value);

  // FrameEx defaults (speechPlayer_frameEx_defaults in src/frame.h).
  {
    const char* st = "speechPlayer_frameEx_t";
    const speechPlayer_frameEx_t& d = speechPlayer_frameEx_defaults;
    printBits(st, "creakiness", d.creakiness);
    printBits(st, "breathiness", d.breathiness);
    printBits(st, "jitter", d.jitter);
    printBits(st, "shimmer", d.shimmer);
    printBits(st, "sharpness", d.sharpness);
    printBits(st, "endCf1", d.endCf1);
    printBits(st, "endCf2", d.endCf2);
    printBits(st, "endCf3", d.endCf3);
    printBits(st, "endPf1", d.endPf1);
    printBits(st, "endPf2", d.endPf2);
    printBits(st, "endPf3", d.endPf3);
    printBits(st, "fujisakiEnabled", d.fujisakiEnabled);
    printBits(st, "fujisakiReset", d.fujisakiReset);
    printBits(st, "fujisakiPhraseAmp", d.fujisakiPhraseAmp);
    printBits(st, "fujisakiPhraseLen", d.fujisakiPhraseLen);
    printBits(st, "fujisakiAccentAmp", d.fujisakiAccentAmp);
    printBits(st, "fujisakiAccentDur", d.fujisakiAccentDur);
    printBits(st, "fujisakiAccentLen", d.fujisakiAccentLen);
    printBits(st, "transF1Scale", d.transF1Scale);
    printBits(st, "transF2Scale", d.transF2Scale);
    printBits(st, "transF3Scale", d.transF3Scale);
    printBits(st, "transNasalScale", d.transNasalScale);
    printBits(st, "transAmplitudeMode", d.transAmplitudeMode);
    printBits(st, "cf7", d.cf7);
    printBits(st, "cb7", d.cb7);
    printBits(st, "cf8", d.cf8);
    printBits(st, "cb8", d.cb8);
    printBits(st, "transSourceHoldRatio", d.transSourceHoldRatio);
    printBits(st, "transVoicingHoldRatio", d.transVoicingHoldRatio);
    printBits(st, "fricationTiltDb", d.fricationTiltDb);
    printBits(st, "endVoiceAmplitude", d.endVoiceAmplitude);
    printBits(st, "amplitudeOnsetMs", d.amplitudeOnsetMs);
  }

  // VoicingTone defaults (SPEECHPLAYER_VOICINGTONE_DEFAULTS in src/voicingTone.h).
  {
    const char* st = "speechPlayer_voicingTone_t";
    const speechPlayer_voicingTone_t t = SPEECHPLAYER_VOICINGTONE_DEFAULTS;
    printU32(st, "magic", t.magic);
    printU32(st, "structSize", t.structSize);
    printU32(st, "structVersion", t.structVersion);
    printU32(st, "dspVersion", t.dspVersion);
    printBits(st, "voicingPeakPos", t.voicingPeakPos);
    printBits(st, "voicedPreEmphA", t.voicedPreEmphA);
    printBits(st, "voicedPreEmphMix", t.voicedPreEmphMix);
    printBits(st, "highShelfGainDb", t.highShelfGainDb);
    printBits(st, "highShelfFcHz", t.highShelfFcHz);
    printBits(st, "highShelfQ", t.highShelfQ);
    printBits(st, "voicedTiltDbPerOct", t.voicedTiltDbPerOct);
    printBits(st, "noiseGlottalModDepth", t.noiseGlottalModDepth);
    printBits(st, "pitchSyncF1DeltaHz", t.pitchSyncF1DeltaHz);
    printBits(st, "pitchSyncB1DeltaHz", t.pitchSyncB1DeltaHz);
    printBits(st, "speedQuotient", t.speedQuotient);
    printBits(st, "aspirationTiltDbPerOct", t.aspirationTiltDbPerOct);
    printBits(st, "cascadeBwScale", t.cascadeBwScale);
    printBits(st, "tremorDepth", t.tremorDepth);
    printBits(st, "nasalBwScale", t.nasalBwScale);
    printBits(st, "f4FreqScale", t.f4FreqScale);
    printBits(st, "nasalGainScale", t.nasalGainScale);
    printBits(st, "chorusDepth", t.chorusDepth);
    printBits(st, "chorusDetuneHz", t.chorusDetuneHz);
  }

  return 0;
}
