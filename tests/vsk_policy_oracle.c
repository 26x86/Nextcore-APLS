/* Test-only oracle. Compile against the explicitly supplied, unchanged VSK C
 * policy; this line-based fixture input is not a guest/runtime protocol. */
#include "vf_policy.h"
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
int main(void) {
    char hex[129];
    while (scanf("%128s", hex) == 1) {
        struct vf_msg_header decoded, header;
        struct vf_grant_entry table[2] = {0};
        const struct vf_grant_entry *found = NULL;
        uint8_t bytes[64];
        uint32_t header_count, caller, rights, count;
        uint64_t now;
        if (strlen(hex) != 128 || scanf("%" SCNu32 " %" SCNu32 " %" SCNu32 " %" SCNu64 " %" SCNu32,
            &header_count, &caller, &rights, &now, &count) != 5 || count > 2) return 2;
        for (unsigned i = 0; i < 64; ++i) {
            unsigned value;
            if (sscanf(hex + i * 2, "%2x", &value) != 1) return 3;
            bytes[i] = (uint8_t)value;
        }
        /* Host test requires LE only for constructing the policy's typed input.
         * vf_decode_header is tested separately on the original byte stream. */
        const uint16_t endian = 1;
        if (*(const uint8_t *)&endian != 1 || sizeof(header) != 64) return 4;
        memcpy(&header, bytes, sizeof(header));
        for (uint32_t i = 0; i < count; ++i) {
            struct vf_grant_entry *g = &table[i];
            if (scanf("%" SCNu32 " %" SCNu32 " %" SCNu32 " %" SCNu32 " %" SCNu32 " %" SCNu32
                " %" SCNu64 " %" SCNu64 " %" SCNu64 " %" SCNu64 " %" SCNu32 " %" SCNu32,
                &g->cap.slot, &g->cap.generation, &g->cap.holder_cell, &g->cap.object_type,
                &g->cap.rights, &g->cap.state, &g->cap.object_id, &g->cap.valid_from,
                &g->cap.expires_at, &g->bytes, &g->owner_cell, &g->state) != 12) return 5;
        }
        vf_status decode = vf_decode_header(bytes, header_count, &decoded);
        vf_status policy = vf_validate_payload_grant(&header, table, count, caller, rights, now, &found);
        printf("%d %d\n", (int)decode, (int)policy);
    }
    return ferror(stdin) ? 6 : 0;
}
