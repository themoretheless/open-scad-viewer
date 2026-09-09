/* Native smoke for libphotogrammetry_ffi: ABI calls, envelope shape, GPU flag.
 * Build: cc -O2 -o smoke smoke.c -L../../target/release -lphotogrammetry_wasm
 * Run:   DYLD_LIBRARY_PATH=../../target/release ./smoke   (macOS)
 *        LD_LIBRARY_PATH=../../target/release ./smoke     (Linux)
 */
#include "photogrammetry.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures = 0;
static void check(int condition, const char *what) {
    if (!condition) {
        printf("FAIL: %s\n", what);
        failures++;
    }
}

/* 64-bit native: responses come via photo_response_ptr/len, not the packed
 * return value (its pointer field is wasm32-shaped). */
static uintptr_t take_response(uintptr_t packed, size_t *len) {
    (void)packed;
    *len = photo_response_len();
    return photo_response_ptr();
}

int main(void) {
    /* Blank 64x64 photos: accepted at import, rejected at reconstruction. */
    const size_t len = 64 * 64 * 3;
    for (int i = 0; i < 2; i++) {
        uintptr_t buffer = photo_alloc(len);
        check(buffer != 0, "photo_alloc");
        memset((void *)buffer, 128, len);
        uintptr_t response = photo_add(64, 64, 100.0, buffer, len);
        size_t response_len;
        uintptr_t ptr = take_response(response, &response_len);
        const unsigned char *bytes = (const unsigned char *)ptr;
        check(response_len > 8 && memcmp(bytes, "MGV1", 4) == 0 && memchr(bytes, 2, response_len) != NULL,
              "photo_add ok envelope (MGV1, bool true)");
        photo_free(ptr, response_len);
    }
    uintptr_t acceleration = photo_set_acceleration(1);
    size_t acceleration_len;
    uintptr_t ptr = take_response(acceleration, &acceleration_len);
    check(acceleration_len > 0, "photo_set_acceleration(1) accepted");
    photo_free(ptr, acceleration_len);
    check(photo_set_acceleration(2) != 0, "unknown mode returns an envelope");

    uintptr_t sparse = photo_run(1, 0);
    size_t sparse_len;
    uintptr_t sparse_ptr = take_response(sparse, &sparse_len);
    const unsigned char *bytes = (const unsigned char *)sparse_ptr;
    check(sparse_len > 4 && memcmp(bytes, "MGV1", 4) == 0, "sparse response present");
    /* Uniform inputs must fail reconstruction with a readable message. */
    int found = 0;
    for (size_t i = 0; i + 10 <= sparse_len; i++) {
        if (memcmp(bytes + i, "initialize", 10) == 0) { found = 1; break; }
    }
    check(found, "sparse failure envelope is readable");
    photo_free(sparse_ptr, sparse_len);

    uintptr_t cleared = photo_run(0, 0);
    size_t cleared_len;
    uintptr_t cleared_ptr = take_response(cleared, &cleared_len);
    check(cleared_len > 0, "clear response present");
    photo_free(cleared_ptr, cleared_len);

    if (failures) {
        printf("native smoke: %d failures\n", failures);
        return 1;
    }
    printf("native smoke: ok\n");
    return 0;
}
