#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "quirc.h"

static int read_pgm(const char *path, unsigned char **pixels, int *width, int *height) {
    FILE *file = fopen(path, "rb");
    int maximum;
    size_t length;
    if (!file) return -1;
    if (fscanf(file, "P5 %d %d %d", width, height, &maximum) != 3 ||
        *width <= 0 || *height <= 0 || maximum != 255 || fgetc(file) == EOF) {
        fclose(file);
        errno = EINVAL;
        return -1;
    }
    length = (size_t)*width * (size_t)*height;
    *pixels = malloc(length);
    if (!*pixels || fread(*pixels, 1, length, file) != length) {
        free(*pixels);
        *pixels = NULL;
        fclose(file);
        return -1;
    }
    fclose(file);
    return 0;
}

int main(int argc, char **argv) {
    struct quirc *decoder;
    struct quirc_code code;
    struct quirc_data data;
    unsigned char *pixels = NULL;
    unsigned char *image;
    int width;
    int height;
    int image_width;
    int image_height;
    int index;

    if (argc == 2 && strcmp(argv[1], "--version") == 0) {
        puts("quirc-reader quirc 1.2");
        return 0;
    }
    if (argc != 2 || read_pgm(argv[1], &pixels, &width, &height) < 0) {
        fprintf(stderr, "usage: quirc-reader IMAGE.pgm\n");
        return 2;
    }
    decoder = quirc_new();
    if (!decoder || quirc_resize(decoder, width, height) < 0) {
        fprintf(stderr, "failed to allocate quirc decoder\n");
        free(pixels);
        quirc_destroy(decoder);
        return 3;
    }
    image = quirc_begin(decoder, &image_width, &image_height);
    memcpy(image, pixels, (size_t)image_width * (size_t)image_height);
    free(pixels);
    quirc_end(decoder);

    for (index = 0; index < quirc_count(decoder); index++) {
        int byte;
        quirc_decode_error_t error;
        quirc_extract(decoder, index, &code);
        error = quirc_decode(&code, &data);
        if (error != QUIRC_SUCCESS) continue;
        for (byte = 0; byte < data.payload_len; byte++) printf("%02x", data.payload[byte]);
        putchar('\n');
        quirc_destroy(decoder);
        return 0;
    }
    fprintf(stderr, "no decodable QR code found\n");
    quirc_destroy(decoder);
    return 4;
}
