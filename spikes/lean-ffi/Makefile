CC ?= cc
LAKE ?= lake

GTK_CFLAGS := $(shell pkg-config --cflags gtk4)
LEAN_PREFIX := $(shell lean --print-prefix 2>/dev/null)
LEAN_CFLAGS := $(if $(LEAN_PREFIX),-I$(LEAN_PREFIX)/include,)

BUILD_DIR := build/ffi
ARCHIVE := $(BUILD_DIR)/libaporiffi.a
OBJECTS := $(BUILD_DIR)/ffi.o $(BUILD_DIR)/lean_bridge.o

.PHONY: all check-c build run clean

all: build

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/ffi.o: ffi.c ffi.h | $(BUILD_DIR)
	$(CC) -std=c11 -Wall -Wextra -Werror -O2 $(GTK_CFLAGS) -c $< -o $@

$(BUILD_DIR)/lean_bridge.o: lean_bridge.c ffi.h | $(BUILD_DIR)
	$(CC) -std=c11 -Wall -Wextra -O2 $(GTK_CFLAGS) $(LEAN_CFLAGS) -c $< -o $@

$(ARCHIVE): $(OBJECTS)
	ar rcs $@ $^

# Compiles only the Lean-free half, so the GTK code can be checked without Lean.
check-c:
	$(CC) -std=c11 -Wall -Wextra -Werror -fsyntax-only $(GTK_CFLAGS) ffi.c

build: check-c $(ARCHIVE)
	$(LAKE) build

run: build
	$(LAKE) exe aporiaFfi

clean:
	rm -rf build .lake
