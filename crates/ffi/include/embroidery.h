#ifndef EMBROIDERY_H
#define EMBROIDERY_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Engine version, for example "0.1.0". The pointer is stable for the process. */
const char *embroidery_engine_version(void);

/*
 * Compile a StitchPlan JSON document to DST.
 *
 * Return codes:
 *   0  success, *out_ptr is the DST file
 *   1  the plan was rejected; *out_ptr is a UTF-8 message with no trailing NUL
 *   2  a required pointer was NULL
 *
 * Free every non-NULL out_ptr with embroidery_free, including error messages.
 * out_cap is the allocator capacity and must be passed back unchanged.
 */
int embroidery_compile_dst_json(
    const char *plan_json,
    uint8_t **out_ptr,
    size_t *out_len,
    size_t *out_cap);

/* Validate a StitchPlan. Success returns a JSON report in *out_ptr. */
int embroidery_validate_json(
    const char *plan_json,
    uint8_t **out_ptr,
    size_t *out_len,
    size_t *out_cap);

/* Score production risk. Success returns a JSON report in *out_ptr. */
int embroidery_score_json(
    const char *plan_json,
    uint8_t **out_ptr,
    size_t *out_len,
    size_t *out_cap);

/*
 * Generate a stitch plan from an embroidery-project version 1 document.
 * Success returns JSON with the plan, validation, readiness, and DST hex
 * when engineering export is allowed. Machine release is never granted.
 */
int embroidery_generate_project_json(
    const char *project_json,
    uint8_t **out_ptr,
    size_t *out_len,
    size_t *out_cap);

/* Capability manifest. Status 0 returns JSON. Status 2 means a null pointer. */
int embroidery_capabilities_json(
    uint8_t **out_ptr,
    size_t *out_len,
    size_t *out_cap);

void embroidery_free(uint8_t *ptr, size_t len, size_t cap);

#ifdef __cplusplus
}
#endif

#endif
