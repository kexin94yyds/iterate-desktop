#ifndef ITERATE_MACOS_SPEECH_ABI_H
#define ITERATE_MACOS_SPEECH_ABI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct {
    uint64_t control_schema_version;
    uint64_t owner_epoch_hi;
    uint64_t owner_epoch_lo;
    uint64_t control_seq;
    uint64_t session_sequence;
    uint64_t revision;
} SpeechLayerIdentity;

_Static_assert(sizeof(SpeechLayerIdentity) == 48, "SpeechLayerIdentity must be six uint64_t words");

static inline bool SpeechLayerIdentityEqual(SpeechLayerIdentity left, SpeechLayerIdentity right) {
    return left.control_schema_version == right.control_schema_version &&
           left.owner_epoch_hi == right.owner_epoch_hi &&
           left.owner_epoch_lo == right.owner_epoch_lo &&
           left.control_seq == right.control_seq &&
           left.session_sequence == right.session_sequence &&
           left.revision == right.revision;
}

typedef void (*SpeechBridgeCallback)(SpeechLayerIdentity identity,
                                     const char *event_type,
                                     const char *text,
                                     void *user_data);

void speech_bridge_start(SpeechLayerIdentity identity,
                         SpeechBridgeCallback callback,
                         void *user_data,
                         const char **contextual_strings,
                         size_t contextual_count,
                         bool force_on_device);
void speech_bridge_finish(SpeechLayerIdentity identity);
void speech_bridge_cancel(SpeechLayerIdentity identity);

#endif
