// Code for ESS adapter from https://github.com/Skuzee/ESS-Adapter.

#ifndef __ESS_H__
#define __ESS_H__

#include <stdint.h>

void gc_to_n64(uint8_t coords[2]);

uint16_t triangular_to_linear_index(uint8_t row, uint8_t col, uint8_t size);

void invert_vc(uint8_t coords[2]);

void invert_vc_gc(uint8_t coords[2]);

void invert_vc_n64(int8_t coords[2], uint8_t ucoords[2]);

void normalize_origin(uint8_t coords[2], uint8_t origin[2]);

#endif
